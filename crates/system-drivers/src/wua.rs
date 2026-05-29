//! Windows Update Agent (WUA) COM driver source.
//!
//! Searches `IsInstalled=0 and Type='Driver'` against the Microsoft Update
//! Catalog service (`ServerSelection=ssOthers` + the MU `ServiceID`), falling
//! back to default Windows Update when the catalog service is unavailable.
//! Download + install reuse the same WUA session. Only vendor-signed drivers
//! Microsoft distributes are ever returned — no scraping.

use windows::core::{Interface, BSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Ole::VarUI8FromDec;
use windows::Win32::System::UpdateAgent::{
    orcSucceeded, orcSucceededWithErrors, ssOthers, ssWindowsUpdate, IUpdateCollection,
    IUpdateSession, IWindowsDriverUpdate, ServerSelection, UpdateCollection, UpdateSession,
};

use crate::{
    classify_best, extract_version, ole_date_to_iso, pick_support_url, DriverError, DriverUpdate,
    InstallProgress, InstallReport, InstallStage, UpdateSource,
};

/// Microsoft Update service id (enables the broader Microsoft Update Catalog).
const MICROSOFT_UPDATE_SERVICE_ID: &str = "7971f918-a847-4430-9279-4a52d1efe18d";
const DRIVER_CRITERIA: &str = "IsInstalled=0 and Type='Driver'";

/// Live WUA-backed driver update source.
pub struct WuaSource;

impl UpdateSource for WuaSource {
    fn search(&self) -> Result<Vec<DriverUpdate>, DriverError> {
        match unsafe { search_with(ssOthers, Some(MICROSOFT_UPDATE_SERVICE_ID)) } {
            Ok(v) => Ok(v),
            Err(_) => unsafe { search_with(ssWindowsUpdate, None) }
                .map_err(|e| DriverError::Search(e.to_string())),
        }
    }

    fn install(
        &self,
        update_id: &str,
        on_progress: &mut dyn FnMut(InstallProgress),
    ) -> Result<InstallReport, DriverError> {
        unsafe { install_impl(update_id, on_progress) }
    }
}

unsafe fn init_com() {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
}

unsafe fn make_session() -> windows::core::Result<IUpdateSession> {
    init_com();
    CoCreateInstance(&UpdateSession, None, CLSCTX_INPROC_SERVER)
}

unsafe fn search_with(
    server: ServerSelection,
    service_id: Option<&str>,
) -> windows::core::Result<Vec<DriverUpdate>> {
    let session = make_session()?;
    let searcher = session.CreateUpdateSearcher()?;
    searcher.SetServerSelection(server)?;
    if let Some(id) = service_id {
        searcher.SetServiceID(&BSTR::from(id))?;
    }
    let result = searcher.Search(&BSTR::from(DRIVER_CRITERIA))?;
    let coll = result.Updates()?;
    let count = coll.Count()?;

    let mut out = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        let update = coll.get_Item(i)?;
        let title = update.Title().map(|b| b.to_string()).unwrap_or_default();
        let identity = update.Identity()?;
        let update_id = format!("{}:{}", identity.UpdateID()?, identity.RevisionNumber()?);
        let size_bytes = update
            .MaxDownloadSize()
            .ok()
            .and_then(|d| VarUI8FromDec(&d).ok())
            .unwrap_or(0);
        let mut url_candidates: Vec<String> = Vec::new();
        if let Ok(more) = update.MoreInfoUrls() {
            if let Ok(count) = more.Count() {
                for j in 0..count {
                    if let Ok(u) = more.get_Item(j) {
                        url_candidates.push(u.to_string());
                    }
                }
            }
        }
        if let Ok(s) = update.SupportUrl() {
            url_candidates.push(s.to_string());
        }
        let support_url = pick_support_url(&url_candidates);

        let (provider, class_raw, hardware_id, driver_date) =
            match update.cast::<IWindowsDriverUpdate>() {
                Ok(drv) => (
                    drv.DriverProvider()
                        .map(|b| b.to_string())
                        .unwrap_or_default(),
                    drv.DriverClass().map(|b| b.to_string()).unwrap_or_default(),
                    drv.DriverHardwareID()
                        .ok()
                        .map(|b| b.to_string())
                        .filter(|s| !s.is_empty()),
                    drv.DriverVerDate().ok().and_then(ole_date_to_iso),
                ),
                Err(_) => (String::new(), String::new(), None, None),
            };

        out.push(DriverUpdate {
            update_id,
            class: classify_best(&class_raw, &title),
            provider,
            driver_version: extract_version(&title),
            driver_date,
            hardware_id,
            size_bytes,
            target_device: None,
            current_version: None,
            target_inf: None,
            target_hardware_id: None,
            support_url,
            title,
        });
    }
    Ok(out)
}

unsafe fn install_impl(
    update_id: &str,
    on_progress: &mut dyn FnMut(InstallProgress),
) -> Result<InstallReport, DriverError> {
    let map_err = |e: windows::core::Error| DriverError::Install(e.to_string());

    let session = make_session().map_err(map_err)?;
    let searcher = session.CreateUpdateSearcher().map_err(map_err)?;
    let _ = searcher.SetServerSelection(ssOthers);
    let _ = searcher.SetServiceID(&BSTR::from(MICROSOFT_UPDATE_SERVICE_ID));
    let result = searcher
        .Search(&BSTR::from(DRIVER_CRITERIA))
        .map_err(map_err)?;
    let found = result.Updates().map_err(map_err)?;
    let count = found.Count().map_err(map_err)?;

    let mut target = None;
    for i in 0..count {
        let u = found.get_Item(i).map_err(map_err)?;
        let id = u.Identity().map_err(map_err)?;
        let this = format!(
            "{}:{}",
            id.UpdateID().map_err(map_err)?,
            id.RevisionNumber().map_err(map_err)?
        );
        if this == update_id {
            target = Some(u);
            break;
        }
    }
    let update = target.ok_or_else(|| DriverError::NotFound(update_id.to_string()))?;
    let _ = update.AcceptEula();

    let coll: IUpdateCollection =
        CoCreateInstance(&UpdateCollection, None, CLSCTX_INPROC_SERVER).map_err(map_err)?;
    coll.Add(&update).map_err(map_err)?;

    on_progress(InstallProgress {
        stage: InstallStage::Downloading,
        message: "Downloading driver from Windows Update…".to_string(),
        fraction: None,
    });
    let downloader = session.CreateUpdateDownloader().map_err(map_err)?;
    downloader.SetUpdates(&coll).map_err(map_err)?;
    downloader.Download().map_err(map_err)?;

    on_progress(InstallProgress {
        stage: InstallStage::Installing,
        message: "Installing driver…".to_string(),
        fraction: None,
    });
    let installer = session.CreateUpdateInstaller().map_err(map_err)?;
    installer.SetUpdates(&coll).map_err(map_err)?;
    let res = installer.Install().map_err(map_err)?;

    let code = res.ResultCode().map_err(map_err)?;
    let reboot_required = res.RebootRequired().map(|b| b.0 != 0).unwrap_or(false);
    let success = code == orcSucceeded || code == orcSucceededWithErrors;

    Ok(InstallReport {
        success,
        reboot_required,
        result_code: code.0,
        message: if success {
            "Driver installed successfully.".to_string()
        } else {
            format!("Windows Update returned result code {}.", code.0)
        },
    })
}
