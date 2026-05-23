use crate::error::{AppError, AppResult};
use crate::state::AppState;
use dll_catalog::{Catalog, Release};
use serde::Serialize;
use std::collections::HashMap;
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
pub async fn refresh_catalog(state: State<'_, AppState>) -> AppResult<()> {
    let http = state.http.clone();
    let cache_path = state.catalog_cache_path.read().clone();
    let catalog = match cache_path {
        Some(p) => Catalog::fetch_with_cache(&http, &p).await?,
        None => Catalog::fetch(&http).await?,
    };
    *state.catalog.write() = Some(catalog);
    Ok(())
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
            for r in &entry.releases {
                let key = shas_key(vendor, family, &r.filename);
                let candidate = (r.version_packed, r.sha256.clone());
                match out.get(&key) {
                    Some(existing) if existing.0 >= r.version_packed => {}
                    _ => {
                        out.insert(key, candidate);
                    }
                }
            }
        }
    }
    Ok(out.into_iter().map(|(k, v)| (k, v.1)).collect())
}
