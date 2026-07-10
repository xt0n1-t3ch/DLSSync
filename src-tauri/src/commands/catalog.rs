use crate::error::{AppError, AppResult};
use crate::state::AppState;
use dll_catalog::{manifest_public_key_fingerprint, Catalog, Release};
use dlssync_application::product_config;
use dlssync_contracts::{
    CatalogDelta, CatalogProvenance, CatalogRefreshResult, CatalogRefreshTrigger, CatalogStatus,
    OperationActor, OperationKind, OperationRecord, OperationStatus,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct CatalogSummary {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub vendors: Vec<VendorSummary>,
    pub incompatible_games: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct VendorSummary {
    pub vendor: String,
    pub families: Vec<FamilySummary>,
}

#[derive(Debug, Serialize)]
pub struct FamilySummary {
    pub family: String,
    pub latest: String,
    pub release_count: usize,
}

#[tauri::command]
pub async fn refresh_catalog(
    state: State<'_, AppState>,
    trigger: Option<CatalogRefreshTrigger>,
) -> AppResult<CatalogRefreshResult> {
    let started = Instant::now();
    let trigger = trigger.unwrap_or(CatalogRefreshTrigger::Automatic);
    let policy = *state.distribution_policy.read();
    let current = state.catalog.read().clone();
    let config = product_config().map_err(|error| AppError::Other(error.to_string()))?;

    if !policy.permits_catalog_refresh(trigger) {
        let provenance =
            state.catalog_provenance.read().clone().unwrap_or_else(|| {
                provenance_for_current(&config, current.as_ref(), trigger, None)
            });
        let result = CatalogRefreshResult {
            refreshed: false,
            blocked_by_policy: true,
            provenance,
            delta: empty_delta(),
        };
        append_refresh_journal(&state, trigger, &result, started.elapsed(), None)?;
        return Ok(result);
    }

    let cache_path = state
        .catalog_cache_path
        .read()
        .clone()
        .ok_or_else(|| AppError::Other("catalog cache path is unavailable".into()))?;
    let fetched = Catalog::fetch_verified_with_cache_from(
        &state.http_catalog,
        &cache_path,
        &config.catalog.canonical_manifest,
        current.as_ref().map(|catalog| catalog.generated_at),
    )
    .await?;
    let delta = catalog_delta(current.as_ref(), &fetched);
    let provenance = provenance_for_current(&config, Some(&fetched), trigger, None);
    persist_provenance(&state, &provenance)?;
    *state.catalog.write() = Some(fetched);
    *state.catalog_provenance.write() = Some(provenance.clone());

    let result = CatalogRefreshResult {
        refreshed: true,
        blocked_by_policy: false,
        provenance,
        delta,
    };
    append_refresh_journal(&state, trigger, &result, started.elapsed(), None)?;
    Ok(result)
}

#[tauri::command]
pub async fn catalog_status(state: State<'_, AppState>) -> AppResult<CatalogStatus> {
    let policy = *state.distribution_policy.read();
    let current = state.catalog.read();
    let config = product_config().map_err(|error| AppError::Other(error.to_string()))?;
    let provenance = state.catalog_provenance.read().clone().unwrap_or_else(|| {
        provenance_for_current(
            &config,
            current.as_ref(),
            CatalogRefreshTrigger::Automatic,
            None,
        )
    });
    Ok(CatalogStatus {
        distribution: policy.channel,
        install_mode: policy.install_mode,
        automatic_refresh_enabled: policy.automatic_catalog_refresh,
        manual_refresh_enabled: policy.manual_catalog_refresh,
        app_updates_enabled: policy.app_updates,
        provenance,
    })
}

#[tauri::command]
pub async fn catalog_summary(state: State<'_, AppState>) -> AppResult<CatalogSummary> {
    let guard = state.catalog.read();
    let catalog = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("catalog not loaded".into()))?;
    let vendors = catalog
        .vendors
        .iter()
        .map(|(vendor, families)| VendorSummary {
            vendor: vendor.clone(),
            families: families
                .iter()
                .map(|(family, entry)| FamilySummary {
                    family: family.clone(),
                    latest: entry.latest.clone(),
                    release_count: entry.releases.len(),
                })
                .collect(),
        })
        .collect();
    Ok(CatalogSummary {
        generated_at: catalog.generated_at,
        vendors,
        incompatible_games: catalog.incompatible_games.clone(),
    })
}

#[tauri::command]
pub async fn list_releases(
    state: State<'_, AppState>,
    vendor: String,
    family: String,
) -> AppResult<Vec<Release>> {
    let guard = state.catalog.read();
    let catalog = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("catalog not loaded".into()))?;
    Ok(catalog.releases(&vendor, &family))
}

pub fn shas_key(vendor: &str, family: &str, filename: &str) -> String {
    format!(
        "{}::{}::{}",
        vendor.to_ascii_lowercase(),
        family.to_ascii_lowercase(),
        filename.to_ascii_lowercase()
    )
}

#[tauri::command]
pub async fn catalog_latest_shas(state: State<'_, AppState>) -> AppResult<HashMap<String, String>> {
    let guard = state.catalog.read();
    let catalog = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("catalog not loaded".into()))?;
    let mut out: HashMap<String, (u64, String)> = HashMap::new();
    for (vendor, families) in &catalog.vendors {
        for (family, entry) in families {
            for release in &entry.releases {
                let key = shas_key(vendor, family, &release.filename);
                let candidate = (release.version_packed, release.sha256.clone());
                match out.get(&key) {
                    Some(existing) if existing.0 >= release.version_packed => {}
                    _ => {
                        out.insert(key, candidate);
                    }
                }
            }
        }
    }
    Ok(out.into_iter().map(|(key, value)| (key, value.1)).collect())
}

pub fn load_persisted_provenance(path: &Path) -> Option<CatalogProvenance> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

pub fn provenance_for_current(
    config: &dlssync_application::ProductConfig,
    catalog: Option<&Catalog>,
    trigger: CatalogRefreshTrigger,
    source_commit: Option<String>,
) -> CatalogProvenance {
    let now = chrono::Utc::now().to_rfc3339();
    CatalogProvenance {
        manifest_url: config.catalog.canonical_manifest.clone(),
        manifest_repository: config.product.manifest_repository.clone(),
        generated_at: catalog
            .map(|value| value.generated_at.to_rfc3339())
            .unwrap_or_else(|| now.clone()),
        checked_at: now,
        signature_verified: catalog.is_some(),
        public_key_fingerprint: manifest_public_key_fingerprint(),
        source_commit,
        trigger,
    }
}

fn persist_provenance(state: &AppState, provenance: &CatalogProvenance) -> AppResult<()> {
    let path = state
        .paths
        .read()
        .as_ref()
        .map(|paths| paths.catalog_metadata.clone())
        .ok_or_else(|| AppError::Other("catalog metadata path is unavailable".into()))?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Other("catalog metadata path has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged
        .write_all(&serde_json::to_vec_pretty(provenance).map_err(|error| {
            AppError::Other(format!("serialize catalog provenance: {error}"))
        })?)?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn append_refresh_journal(
    state: &AppState,
    trigger: CatalogRefreshTrigger,
    result: &CatalogRefreshResult,
    duration: std::time::Duration,
    error: Option<String>,
) -> AppResult<()> {
    let actor = match trigger {
        CatalogRefreshTrigger::Automatic => OperationActor::Background,
        CatalogRefreshTrigger::ManualUser => OperationActor::Gui,
    };
    let status = if error.is_some() {
        OperationStatus::Failed
    } else {
        OperationStatus::Succeeded
    };
    let details = BTreeMap::from([
        ("trigger".into(), format!("{trigger:?}")),
        (
            "blocked_by_policy".into(),
            result.blocked_by_policy.to_string(),
        ),
        (
            "signature_verified".into(),
            result.provenance.signature_verified.to_string(),
        ),
        ("added".into(), result.delta.added.to_string()),
        ("updated".into(), result.delta.updated.to_string()),
        ("removed".into(), result.delta.removed.to_string()),
    ]);
    if let Some(journal) = state.journal.read().as_ref() {
        journal.append(&OperationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            actor,
            kind: OperationKind::CatalogRefresh,
            status,
            target: Some(result.provenance.manifest_repository.clone()),
            summary: if result.blocked_by_policy {
                "Catalog refresh skipped by distribution policy".into()
            } else {
                "Signed catalog refreshed".into()
            },
            details,
            duration_ms: Some(duration.as_millis().min(u128::from(u32::MAX)) as u32),
            backup_id: None,
            error,
        })?;
    }
    Ok(())
}

fn latest_release_versions(catalog: &Catalog) -> BTreeMap<String, u64> {
    let mut versions: BTreeMap<String, u64> = BTreeMap::new();
    for (vendor, families) in &catalog.vendors {
        for (family, entry) in families {
            for release in &entry.releases {
                let key = shas_key(vendor, family, &release.filename);
                versions
                    .entry(key)
                    .and_modify(|current| *current = (*current).max(release.version_packed))
                    .or_insert(release.version_packed);
            }
        }
    }
    versions
}

fn catalog_delta(before: Option<&Catalog>, after: &Catalog) -> CatalogDelta {
    let before = before.map(latest_release_versions).unwrap_or_default();
    let after = latest_release_versions(after);
    CatalogDelta {
        added: after
            .keys()
            .filter(|key| !before.contains_key(*key))
            .count() as u32,
        updated: after
            .iter()
            .filter(|(key, version)| before.get(*key).is_some_and(|old| old != *version))
            .count() as u32,
        removed: before
            .keys()
            .filter(|key| !after.contains_key(*key))
            .count() as u32,
    }
}

const fn empty_delta() -> CatalogDelta {
    CatalogDelta {
        added: 0,
        updated: 0,
        removed: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dll_catalog::{FamilyEntry, Release};

    fn release(filename: &str, version_packed: u64) -> Release {
        Release {
            version: version_packed.to_string(),
            version_packed,
            filename: filename.into(),
            sha256: "a".repeat(64),
            size_bytes: 1,
            signed: true,
            released_at: chrono::Utc::now(),
            source: "https://vendor.example/file".into(),
            cdn_url: "https://vendor.example/file".into(),
            release_notes: None,
            signature_subject: None,
            channel: "stable".into(),
            is_dev: false,
            min_driver: None,
            hash_algorithm: "sha256".into(),
            zip_entry: None,
        }
    }

    fn catalog(files: &[(&str, u64)]) -> Catalog {
        Catalog {
            schema_version: 1,
            generated_at: chrono::Utc::now(),
            vendors: BTreeMap::from([(
                "nvidia".into(),
                BTreeMap::from([(
                    "dlss_sr".into(),
                    FamilyEntry {
                        latest: files
                            .last()
                            .map(|(_, version)| version.to_string())
                            .unwrap_or_default(),
                        releases: files
                            .iter()
                            .map(|(filename, version)| release(filename, *version))
                            .collect(),
                    },
                )]),
            )]),
            incompatible_games: Vec::new(),
            anticheat: None,
            anti_cheat_binaries: Vec::new(),
        }
    }

    #[test]
    fn delta_distinguishes_added_updated_and_removed_files() {
        let before = catalog(&[("old.dll", 1), ("updated.dll", 1)]);
        let after = catalog(&[("updated.dll", 2), ("new.dll", 1)]);
        assert_eq!(
            catalog_delta(Some(&before), &after),
            CatalogDelta {
                added: 1,
                updated: 1,
                removed: 1,
            }
        );
    }
}
