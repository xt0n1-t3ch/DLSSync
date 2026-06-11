use crate::settings::{DrsSetting, OverrideScope};
use std::ffi::c_void;
use std::mem::{size_of, transmute, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

type Status = i32;
const NVAPI_OK: Status = 0;
const NVAPI_INVALID_USER_PRIVILEGE: Status = -137;
type NvHandle = *mut c_void;

const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_DRS_CREATE_SESSION: u32 = 0x0694_D52E;
const ID_DRS_LOAD_SETTINGS: u32 = 0x375D_BD6B;
const ID_DRS_GET_BASE_PROFILE: u32 = 0xDA84_66A0;
const ID_DRS_GET_PROFILE_INFO: u32 = 0x61CD_6FD6;
const ID_DRS_SET_SETTING: u32 = 0x577D_D202;
const ID_DRS_GET_SETTING: u32 = 0x73BF_8338;
const ID_DRS_SAVE_SETTINGS: u32 = 0xFCBC_7E14;
const ID_DRS_CREATE_PROFILE: u32 = 0xCC17_6068;
const ID_DRS_CREATE_APPLICATION: u32 = 0x4347_A9DE;
const ID_DRS_FIND_APPLICATION_BY_NAME: u32 = 0xEEE5_66B2;
const ID_DRS_RESTORE_DEFAULT_SETTING: u32 = 0x53F0_381E;
const ID_DRS_DESTROY_SESSION: u32 = 0xDAD9_CFF8;

const UNICODE_STRING_MAX: usize = 2048;
const NVAPI_BINARY_DATA_MAX: usize = 4096;
const NVDRS_DWORD_TYPE: u32 = 0;
const NVDRS_PROFILE_VER_NUMBER: u32 = 1;
const NVDRS_SETTING_VER_NUMBER: u32 = 1;
const NVDRS_APPLICATION_VER_NUMBER: u32 = 4;

type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *const c_void;
type InitializeFn = unsafe extern "C" fn() -> Status;
type CreateSessionFn = unsafe extern "C" fn(*mut NvHandle) -> Status;
type LoadSettingsFn = unsafe extern "C" fn(NvHandle) -> Status;
type GetBaseProfileFn = unsafe extern "C" fn(NvHandle, *mut NvHandle) -> Status;
type GetProfileInfoFn = unsafe extern "C" fn(NvHandle, NvHandle, *mut NvdrsProfile) -> Status;
type SetSettingFn = unsafe extern "C" fn(NvHandle, NvHandle, *const NvdrsSetting) -> Status;
type GetSettingFn = unsafe extern "C" fn(NvHandle, NvHandle, u32, *mut NvdrsSetting) -> Status;
type SaveSettingsFn = unsafe extern "C" fn(NvHandle) -> Status;
type CreateProfileFn = unsafe extern "C" fn(NvHandle, *const NvdrsProfile, *mut NvHandle) -> Status;
type CreateApplicationFn =
    unsafe extern "C" fn(NvHandle, NvHandle, *mut NvdrsApplicationV4) -> Status;
type FindApplicationByNameFn =
    unsafe extern "C" fn(NvHandle, *const u16, *mut NvHandle, *mut NvdrsApplicationV4) -> Status;
type RestoreDefaultSettingFn = unsafe extern "C" fn(NvHandle, NvHandle, u32) -> Status;
type DestroySessionFn = unsafe extern "C" fn(NvHandle) -> Status;

#[repr(C)]
struct NvdrsProfile {
    version: u32,
    profile_name: [u16; UNICODE_STRING_MAX],
    gpu_support: u32,
    is_predefined: u32,
    num_of_apps: u32,
    num_of_settings: u32,
}

#[repr(C)]
struct NvdrsSettingValue {
    u32_value: u32,
    _binary_tail: [u8; NVAPI_BINARY_DATA_MAX],
}

#[repr(C)]
struct NvdrsSetting {
    version: u32,
    setting_name: [u16; UNICODE_STRING_MAX],
    setting_id: u32,
    setting_type: u32,
    setting_location: u32,
    is_current_predefined: u32,
    is_predefined_valid: u32,
    predefined_value: NvdrsSettingValue,
    current_value: NvdrsSettingValue,
}

#[repr(C)]
struct NvdrsApplicationV4 {
    version: u32,
    is_predefined: u32,
    app_name: [u16; UNICODE_STRING_MAX],
    user_friendly_name: [u16; UNICODE_STRING_MAX],
    launcher: [u16; UNICODE_STRING_MAX],
    file_in_folder: [u16; UNICODE_STRING_MAX],
    flags: u32,
    command_line: [u16; UNICODE_STRING_MAX],
}

#[derive(Debug, Clone)]
pub struct BaseProfileInfo {
    pub name: String,
    pub num_apps: u32,
    pub num_settings: u32,
    pub struct_version: u32,
}

fn struct_version<T>(number: u32) -> u32 {
    (size_of::<T>() as u32) | (number << 16)
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn wide_into(dst: &mut [u16], value: &str) {
    let limit = dst.len().saturating_sub(1);
    for (slot, ch) in dst.iter_mut().zip(value.encode_utf16().take(limit)) {
        *slot = ch;
    }
}

fn utf16_until_nul(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn exe_basename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// Ordered DRS application-name lookup candidates for an executable path.
///
/// A per-game DRS profile is keyed by the name passed to
/// `NvAPI_DRS_FindApplicationByName`. We prefer the FULL executable path so two
/// games sharing an exe basename (e.g. `launcher.exe`) get distinct profiles,
/// matching the full-path semantics implied by [`OverrideScope::PerGame`]. The
/// bare basename is kept only as a last-resort fallback so profiles created by
/// NVIDIA tooling (which historically register apps by basename) still resolve.
///
/// The basename is omitted when it equals the full path (a path with no
/// directory component) to avoid a redundant second lookup with the same key.
fn app_lookup_candidates(exe_path: &str) -> Vec<String> {
    let basename = exe_basename(exe_path);
    let mut candidates = vec![exe_path.to_string()];
    if basename != exe_path {
        candidates.push(basename);
    }
    candidates
}

struct DrsFns {
    get_base_profile: GetBaseProfileFn,
    get_profile_info: GetProfileInfoFn,
    set_setting: SetSettingFn,
    get_setting: GetSettingFn,
    save_settings: SaveSettingsFn,
    create_profile: CreateProfileFn,
    create_application: CreateApplicationFn,
    find_application_by_name: FindApplicationByNameFn,
    restore_default_setting: RestoreDefaultSettingFn,
    destroy_session: DestroySessionFn,
}

struct Drs {
    lib: HMODULE,
    session: NvHandle,
    fns: DrsFns,
}

impl Drs {
    fn open() -> Result<Self, String> {
        unsafe {
            let lib = LoadLibraryW(wide("nvapi64.dll").as_ptr());
            if lib.is_null() {
                return Err("nvapi64.dll failed to load — NVIDIA driver not present".to_string());
            }
            match Self::init(lib) {
                Ok(drs) => Ok(drs),
                Err(error) => {
                    let _ = FreeLibrary(lib);
                    Err(error)
                }
            }
        }
    }

    unsafe fn init(lib: HMODULE) -> Result<Self, String> {
        let proc = GetProcAddress(lib, c"nvapi_QueryInterface".as_ptr().cast())
            .ok_or("nvapi_QueryInterface not exported")?;
        let query = transmute::<unsafe extern "system" fn() -> isize, QueryInterfaceFn>(proc);
        let resolve = |id: u32, label: &str| -> Result<*const c_void, String> {
            let ptr = query(id);
            if ptr.is_null() {
                Err(format!("QueryInterface({label}) returned null"))
            } else {
                Ok(ptr)
            }
        };

        let initialize =
            transmute::<*const c_void, InitializeFn>(resolve(ID_INITIALIZE, "Initialize")?);
        let create_session = transmute::<*const c_void, CreateSessionFn>(resolve(
            ID_DRS_CREATE_SESSION,
            "DRS_CreateSession",
        )?);
        let load_settings = transmute::<*const c_void, LoadSettingsFn>(resolve(
            ID_DRS_LOAD_SETTINGS,
            "DRS_LoadSettings",
        )?);
        let fns = DrsFns {
            get_base_profile: transmute::<*const c_void, GetBaseProfileFn>(resolve(
                ID_DRS_GET_BASE_PROFILE,
                "DRS_GetBaseProfile",
            )?),
            get_profile_info: transmute::<*const c_void, GetProfileInfoFn>(resolve(
                ID_DRS_GET_PROFILE_INFO,
                "DRS_GetProfileInfo",
            )?),
            set_setting: transmute::<*const c_void, SetSettingFn>(resolve(
                ID_DRS_SET_SETTING,
                "DRS_SetSetting",
            )?),
            get_setting: transmute::<*const c_void, GetSettingFn>(resolve(
                ID_DRS_GET_SETTING,
                "DRS_GetSetting",
            )?),
            save_settings: transmute::<*const c_void, SaveSettingsFn>(resolve(
                ID_DRS_SAVE_SETTINGS,
                "DRS_SaveSettings",
            )?),
            create_profile: transmute::<*const c_void, CreateProfileFn>(resolve(
                ID_DRS_CREATE_PROFILE,
                "DRS_CreateProfile",
            )?),
            create_application: transmute::<*const c_void, CreateApplicationFn>(resolve(
                ID_DRS_CREATE_APPLICATION,
                "DRS_CreateApplication",
            )?),
            find_application_by_name: transmute::<*const c_void, FindApplicationByNameFn>(resolve(
                ID_DRS_FIND_APPLICATION_BY_NAME,
                "DRS_FindApplicationByName",
            )?),
            restore_default_setting: transmute::<*const c_void, RestoreDefaultSettingFn>(resolve(
                ID_DRS_RESTORE_DEFAULT_SETTING,
                "DRS_RestoreProfileDefaultSetting",
            )?),
            destroy_session: transmute::<*const c_void, DestroySessionFn>(resolve(
                ID_DRS_DESTROY_SESSION,
                "DRS_DestroySession",
            )?),
        };

        let status = initialize();
        if status != NVAPI_OK {
            return Err(format!("NvAPI_Initialize -> {status}"));
        }
        let mut session: NvHandle = null_mut();
        let status = create_session(&mut session);
        if status != NVAPI_OK {
            return Err(format!("NvAPI_DRS_CreateSession -> {status}"));
        }
        let status = load_settings(session);
        if status != NVAPI_OK {
            (fns.destroy_session)(session);
            return Err(format!("NvAPI_DRS_LoadSettings -> {status}"));
        }
        Ok(Self { lib, session, fns })
    }

    unsafe fn base_profile(&self) -> Result<NvHandle, String> {
        let mut profile: NvHandle = null_mut();
        let status = (self.fns.get_base_profile)(self.session, &mut profile);
        if status != NVAPI_OK {
            return Err(format!("NvAPI_DRS_GetBaseProfile -> {status}"));
        }
        Ok(profile)
    }

    unsafe fn profile_for(&self, scope: &OverrideScope) -> Result<NvHandle, String> {
        match scope {
            OverrideScope::Global => self.base_profile(),
            OverrideScope::PerGame { executable_path } => self.find_or_create_app(executable_path),
        }
    }

    /// Read-only profile resolution: never creates a profile. A per-game profile
    /// that does not exist yet resolves to `None` so a READ stays side-effect-free
    /// (opening the override UI must not inject a phantom app profile).
    unsafe fn profile_for_read(&self, scope: &OverrideScope) -> Result<Option<NvHandle>, String> {
        match scope {
            OverrideScope::Global => Ok(Some(self.base_profile()?)),
            OverrideScope::PerGame { executable_path } => self.find_app(executable_path),
        }
    }

    unsafe fn find_app_by_name(&self, name: &str) -> Option<NvHandle> {
        let name_wide = wide(name);
        let mut profile: NvHandle = null_mut();
        let mut app: NvdrsApplicationV4 = zeroed();
        app.version = struct_version::<NvdrsApplicationV4>(NVDRS_APPLICATION_VER_NUMBER);
        let status = (self.fns.find_application_by_name)(
            self.session,
            name_wide.as_ptr(),
            &mut profile,
            &mut app,
        );
        (status == NVAPI_OK && !profile.is_null()).then_some(profile)
    }

    /// Resolves the per-game profile for an executable, preferring a full-path
    /// match and falling back to the bare basename. See [`app_lookup_candidates`]
    /// for why the full path wins (basename collisions between distinct games).
    unsafe fn find_app(&self, exe_path: &str) -> Result<Option<NvHandle>, String> {
        Ok(app_lookup_candidates(exe_path)
            .iter()
            .find_map(|name| self.find_app_by_name(name)))
    }

    unsafe fn find_or_create_app(&self, exe_path: &str) -> Result<NvHandle, String> {
        if let Some(profile) = self.find_app(exe_path)? {
            return Ok(profile);
        }

        // The app is keyed by its FULL path so it never collides with another
        // game's identically named exe; the basename stays as the human label.
        let friendly = exe_basename(exe_path);
        let mut profile_info: NvdrsProfile = zeroed();
        profile_info.version = struct_version::<NvdrsProfile>(NVDRS_PROFILE_VER_NUMBER);
        wide_into(&mut profile_info.profile_name, exe_path);
        let mut profile: NvHandle = null_mut();
        let status = (self.fns.create_profile)(self.session, &profile_info, &mut profile);
        if status != NVAPI_OK {
            return Err(format!("NvAPI_DRS_CreateProfile -> {status}"));
        }

        let mut app: NvdrsApplicationV4 = zeroed();
        app.version = struct_version::<NvdrsApplicationV4>(NVDRS_APPLICATION_VER_NUMBER);
        wide_into(&mut app.app_name, exe_path);
        wide_into(&mut app.user_friendly_name, &friendly);
        let status = (self.fns.create_application)(self.session, profile, &mut app);
        if status != NVAPI_OK {
            return Err(format!("NvAPI_DRS_CreateApplication -> {status}"));
        }
        Ok(profile)
    }

    unsafe fn set_dword(&self, profile: NvHandle, id: u32, value: u32) -> Result<(), Status> {
        let mut setting: NvdrsSetting = zeroed();
        setting.version = struct_version::<NvdrsSetting>(NVDRS_SETTING_VER_NUMBER);
        setting.setting_id = id;
        setting.setting_type = NVDRS_DWORD_TYPE;
        setting.current_value.u32_value = value;
        let status = (self.fns.set_setting)(self.session, profile, &setting);
        if status == NVAPI_OK {
            Ok(())
        } else {
            Err(status)
        }
    }

    unsafe fn get_dword(&self, profile: NvHandle, id: u32) -> Option<u32> {
        let mut setting: NvdrsSetting = zeroed();
        setting.version = struct_version::<NvdrsSetting>(NVDRS_SETTING_VER_NUMBER);
        let status = (self.fns.get_setting)(self.session, profile, id, &mut setting);
        if status == NVAPI_OK && setting.setting_type == NVDRS_DWORD_TYPE {
            Some(setting.current_value.u32_value)
        } else {
            None
        }
    }

    unsafe fn save(&self) -> Result<(), String> {
        let status = (self.fns.save_settings)(self.session);
        if status != NVAPI_OK {
            return Err(format!("NvAPI_DRS_SaveSettings -> {status}"));
        }
        Ok(())
    }
}

impl Drop for Drs {
    fn drop(&mut self) {
        unsafe {
            (self.fns.destroy_session)(self.session);
            let _ = FreeLibrary(self.lib);
        }
    }
}

pub fn read_base_profile() -> Result<BaseProfileInfo, String> {
    let drs = Drs::open()?;
    unsafe {
        let profile = drs.base_profile()?;
        let mut info: NvdrsProfile = zeroed();
        let version = struct_version::<NvdrsProfile>(NVDRS_PROFILE_VER_NUMBER);
        info.version = version;
        let status = (drs.fns.get_profile_info)(drs.session, profile, &mut info);
        if status != NVAPI_OK {
            return Err(format!(
                "NvAPI_DRS_GetProfileInfo -> {status} (struct_version=0x{version:08X})"
            ));
        }
        Ok(BaseProfileInfo {
            name: utf16_until_nul(&info.profile_name),
            num_apps: info.num_of_apps,
            num_settings: info.num_of_settings,
            struct_version: version,
        })
    }
}

pub fn roundtrip_dword(setting_id: u32, value: u32) -> Result<u32, String> {
    let drs = Drs::open()?;
    unsafe {
        let profile = drs.base_profile()?;
        drs.set_dword(profile, setting_id, value)
            .map_err(|status| format!("NvAPI_DRS_SetSetting(0x{setting_id:08X}) -> {status}"))?;
        drs.get_dword(profile, setting_id)
            .ok_or_else(|| "GetSetting returned no value after SetSetting".to_string())
    }
}

/// Applies every setting it can and returns the ids the driver refused for lack of
/// privilege (`NVAPI_INVALID_USER_PRIVILEGE`) instead of aborting the whole batch —
/// e.g. frame-generation DRS settings require an elevated process, while DLSS
/// Super-Resolution settings do not. Any OTHER NVAPI failure still aborts.
pub fn apply_overrides(scope: &OverrideScope, settings: &[DrsSetting]) -> Result<Vec<u32>, String> {
    let drs = Drs::open()?;
    unsafe {
        let profile = drs.profile_for(scope)?;
        let mut needs_elevation = Vec::new();
        for setting in settings {
            match drs.set_dword(profile, setting.id, setting.value) {
                Ok(()) => {}
                Err(NVAPI_INVALID_USER_PRIVILEGE) => needs_elevation.push(setting.id),
                Err(status) => {
                    return Err(format!(
                        "NvAPI_DRS_SetSetting(0x{:08X}) -> {status}",
                        setting.id
                    ));
                }
            }
        }
        drs.save()?;
        Ok(needs_elevation)
    }
}

pub fn read_overrides(
    scope: &OverrideScope,
    ids: &[u32],
) -> Result<Vec<(u32, Option<u32>)>, String> {
    let drs = Drs::open()?;
    unsafe {
        let Some(profile) = drs.profile_for_read(scope)? else {
            return Ok(ids.iter().map(|&id| (id, None)).collect());
        };
        Ok(ids
            .iter()
            .map(|&id| (id, drs.get_dword(profile, id)))
            .collect())
    }
}

pub fn reset_overrides(scope: &OverrideScope, ids: &[u32]) -> Result<(), String> {
    let drs = Drs::open()?;
    unsafe {
        let Some(profile) = drs.profile_for_read(scope)? else {
            return Ok(());
        };
        for &id in ids {
            let _ = (drs.fns.restore_default_setting)(drs.session, profile, id);
        }
        drs.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_path_is_the_first_lookup_candidate() {
        let path = r"C:\Games\Witcher3\witcher3.exe";
        let candidates = app_lookup_candidates(path);
        assert_eq!(candidates.first().map(String::as_str), Some(path));
    }

    #[test]
    fn basename_is_the_fallback_lookup_candidate() {
        let candidates = app_lookup_candidates(r"C:\Games\Witcher3\witcher3.exe");
        assert_eq!(
            candidates,
            vec![
                r"C:\Games\Witcher3\witcher3.exe".to_string(),
                "witcher3.exe".to_string(),
            ]
        );
    }

    #[test]
    fn two_games_sharing_a_basename_get_distinct_full_path_keys() {
        let a = app_lookup_candidates(r"C:\GameA\launcher.exe");
        let b = app_lookup_candidates(r"C:\GameB\launcher.exe");
        assert_ne!(
            a[0], b[0],
            "full-path keys must differ so profiles never collide"
        );
        assert_eq!(
            a[1], b[1],
            "the shared basename is the deliberate last-resort fallback for both"
        );
    }

    #[test]
    fn bare_basename_path_does_not_emit_a_duplicate_candidate() {
        let candidates = app_lookup_candidates("witcher3.exe");
        assert_eq!(candidates, vec!["witcher3.exe".to_string()]);
    }

    #[test]
    fn empty_path_yields_a_single_empty_candidate() {
        assert_eq!(app_lookup_candidates(""), vec![String::new()]);
    }

    #[test]
    fn forward_slash_path_still_extracts_basename_fallback() {
        let candidates = app_lookup_candidates("D:/Games/sub/game.exe");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0], "D:/Games/sub/game.exe");
        assert_eq!(candidates[1], "game.exe");
    }

    #[test]
    fn fallback_is_suppressed_only_when_it_equals_the_full_path() {
        // A root or drive-only path has no distinct file component, so the
        // candidate list collapses to a single key with no redundant fallback.
        assert_eq!(app_lookup_candidates(r"C:\"), vec![r"C:\".to_string()]);
    }
}
