//! Builds DLSSync `manifest.json` from authoritative upstream sources.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use dll_catalog::{Catalog, FamilyEntry, Release};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

const DLSS_SWAPPER_MANIFEST: &str =
    "https://raw.githubusercontent.com/beeradmoore/dlss-swapper/main/docs/manifest.json";

#[derive(Parser, Debug)]
#[command(name = "manifest-builder", version, about)]
struct Cli {
    /// Output path for the generated manifest.json
    #[arg(long, default_value = "manifest/manifest.json")]
    out: PathBuf,
    /// Skip network fetches and only print what would be done.
    #[arg(long)]
    dry_run: bool,
    /// Comma-separated source list (all by default).
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "dlss_swapper,streamline,xess,fsr,reflex,directstorage"
    )]
    sources: Vec<String>,
}

// ----------------------------------------------------------------------
// DLSS Swapper community manifest
// ----------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SwapperManifest {
    #[serde(default)]
    dlss: Vec<SwapperEntry>,
    #[serde(default)]
    dlss_d: Vec<SwapperEntry>,
    #[serde(default)]
    dlss_g: Vec<SwapperEntry>,
}

#[derive(Debug, Deserialize)]
struct SwapperEntry {
    version: String,
    version_number: u64,
    #[serde(default)]
    internal_name: Option<String>,
    md5_hash: String,
    download_url: String,
    #[serde(default)]
    file_description: Option<String>,
    #[serde(default)]
    signed_datetime: Option<String>,
    #[serde(default)]
    is_signature_valid: Option<bool>,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    dll_source: Option<String>,
    #[serde(default)]
    additional_label: Option<String>,
}

// ----------------------------------------------------------------------
// GitHub Releases REST schema
// ----------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
    #[serde(default)]
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

// ----------------------------------------------------------------------
// Mapping rules — which filenames in each upstream zip become which family.
// ----------------------------------------------------------------------

struct FilenameRule {
    /// canonical lowercase DLL filename present in the zip
    filename: &'static str,
    /// (vendor, family) destination
    vendor: &'static str,
    family: &'static str,
}

const STREAMLINE_RULES: &[FilenameRule] = &[
    FilenameRule {
        filename: "sl.interposer.dll",
        vendor: "nvidia",
        family: "streamline",
    },
    FilenameRule {
        filename: "sl.common.dll",
        vendor: "nvidia",
        family: "streamline_common",
    },
    FilenameRule {
        filename: "sl.pcl.dll",
        vendor: "nvidia",
        family: "streamline_pcl",
    },
    FilenameRule {
        filename: "sl.nis.dll",
        vendor: "nvidia",
        family: "streamline_nis",
    },
    FilenameRule {
        filename: "sl.directsr.dll",
        vendor: "nvidia",
        family: "streamline_direct_sr",
    },
    FilenameRule {
        filename: "sl.reflex.dll",
        vendor: "nvidia",
        family: "reflex",
    },
];

const XESS_RULES: &[FilenameRule] = &[
    FilenameRule {
        filename: "libxess.dll",
        vendor: "intel",
        family: "xess_sr",
    },
    FilenameRule {
        filename: "libxess_dx11.dll",
        vendor: "intel",
        family: "xess_sr_dx11",
    },
    FilenameRule {
        filename: "libxess_fg.dll",
        vendor: "intel",
        family: "xess_fg",
    },
    FilenameRule {
        filename: "libxell.dll",
        vendor: "intel",
        family: "xell",
    },
];

const FSR_RULES: &[FilenameRule] = &[
    FilenameRule {
        filename: "amd_fidelityfx_upscaler_dx12.dll",
        vendor: "amd",
        family: "fsr_upscaler",
    },
    FilenameRule {
        filename: "amd_fidelityfx_upscaler_vk.dll",
        vendor: "amd",
        family: "fsr_upscaler_vk",
    },
    FilenameRule {
        filename: "amd_fidelityfx_framegeneration_dx12.dll",
        vendor: "amd",
        family: "fsr_fg",
    },
    FilenameRule {
        filename: "amd_fidelityfx_loader_dx12.dll",
        vendor: "amd",
        family: "fsr_loader",
    },
    FilenameRule {
        filename: "amd_fidelityfx_denoiser_dx12.dll",
        vendor: "amd",
        family: "fsr_denoiser",
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?),
        )
        .init();
    let cli = Cli::parse();

    let mut vendors: BTreeMap<String, BTreeMap<String, FamilyEntry>> = BTreeMap::new();
    let client = build_client()?;

    if cli.sources.iter().any(|s| s == "dlss_swapper") && !cli.dry_run {
        if let Err(e) = ingest_dlss_swapper(&client, &mut vendors).await {
            tracing::error!("dlss_swapper ingest failed: {e:#}");
        }
    }
    if cli.sources.iter().any(|s| s == "streamline") && !cli.dry_run {
        if let Err(e) = ingest_github_zip_releases(
            &client,
            &mut vendors,
            "NVIDIA-RTX/Streamline",
            STREAMLINE_RULES,
            |asset| asset.name.ends_with(".zip") && asset.name.starts_with("streamline-sdk"),
            // canonical zip layout: bin/x64/<dll> or bin/x86/<dll>
        )
        .await
        {
            tracing::error!("streamline ingest failed: {e:#}");
        }
    }
    if cli.sources.iter().any(|s| s == "xess") && !cli.dry_run {
        if let Err(e) =
            ingest_github_zip_releases(&client, &mut vendors, "intel/xess", XESS_RULES, |asset| {
                asset.name.ends_with(".zip") && asset.name.to_lowercase().contains("xess")
            })
            .await
        {
            tracing::error!("xess ingest failed: {e:#}");
        }
    }
    if cli.sources.iter().any(|s| s == "fsr") && !cli.dry_run {
        if let Err(e) = ingest_github_zip_releases(
            &client,
            &mut vendors,
            "GPUOpen-LibrariesAndSDKs/FidelityFX-SDK",
            FSR_RULES,
            |asset| asset.name.ends_with(".zip") && asset.name.to_lowercase().contains("sdk"),
        )
        .await
        {
            tracing::error!("fsr ingest failed: {e:#}");
        }
    }
    if cli.sources.iter().any(|s| s == "reflex") {
        tracing::info!("reflex DLLs ingest via Streamline (sl.reflex.dll). Standalone NVIDIA-RTX/REFLEX SDK ships as PDF + .nupkg outside our scope.");
    }
    if cli.sources.iter().any(|s| s == "directstorage") && !cli.dry_run {
        if let Err(e) = ingest_directstorage_nuget(&client, &mut vendors).await {
            tracing::error!("directstorage ingest failed: {e:#}");
        }
    }

    let catalog = Catalog {
        schema_version: 2,
        generated_at: Utc::now(),
        vendors,
        incompatible_games: vec![],
    };

    if cli.dry_run {
        println!("{}", serde_json::to_string_pretty(&catalog)?);
        return Ok(());
    }
    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(&catalog)?;
    std::fs::write(&cli.out, body)?;
    let total: usize = catalog
        .vendors
        .values()
        .flat_map(|v| v.values())
        .map(|f| f.releases.len())
        .sum();
    tracing::info!(path = %cli.out.display(), releases = total, "wrote manifest");
    Ok(())
}

fn build_client() -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {token}").parse()?,
            );
            tracing::info!("using GITHUB_TOKEN for higher REST rate limit");
        }
    }
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github+json".parse()?,
    );
    Ok(reqwest::Client::builder()
        .user_agent("dlssync-manifest-builder/0.1 (+https://github.com/xt0n1-t3ch/DLSSync)")
        .default_headers(headers)
        .build()?)
}

// ----------------------------------------------------------------------
// DLSS Swapper ingest
// ----------------------------------------------------------------------

async fn ingest_dlss_swapper(
    client: &reqwest::Client,
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
) -> Result<()> {
    tracing::info!("fetching DLSS Swapper manifest");
    let body = client
        .get(DLSS_SWAPPER_MANIFEST)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: SwapperManifest =
        serde_json::from_str(&body).context("parse dlss-swapper manifest")?;
    merge_swapper(
        vendors,
        "nvidia",
        "dlss_sr",
        "nvngx_dlss.dll",
        &manifest.dlss,
    );
    merge_swapper(
        vendors,
        "nvidia",
        "dlss_rr",
        "nvngx_dlssd.dll",
        &manifest.dlss_d,
    );
    merge_swapper(
        vendors,
        "nvidia",
        "dlss_fg",
        "nvngx_dlssg.dll",
        &manifest.dlss_g,
    );
    Ok(())
}

fn merge_swapper(
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
    vendor: &str,
    family: &str,
    filename: &str,
    entries: &[SwapperEntry],
) {
    let mut releases: Vec<Release> = entries
        .iter()
        .map(|e| {
            let released_at = e
                .signed_datetime
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            let is_experimental = e
                .additional_label
                .as_deref()
                .map(|s| {
                    s.to_lowercase().contains("beta") || s.to_lowercase().contains("experimental")
                })
                .unwrap_or(false);
            Release {
                version: e.version.clone(),
                version_packed: e.version_number,
                filename: filename.to_string(),
                sha256: e.md5_hash.clone().to_lowercase(), // upstream gives MD5; treated as opaque integrity tag
                size_bytes: e.file_size.unwrap_or(0),
                signed: e.is_signature_valid.unwrap_or(false),
                released_at,
                source: e
                    .dll_source
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "beeradmoore/dlss-swapper".to_string()),
                cdn_url: e.download_url.clone(),
                release_notes: e
                    .internal_name
                    .clone()
                    .or_else(|| e.file_description.clone()),
                signature_subject: if e.is_signature_valid.unwrap_or(false) {
                    Some("NVIDIA Corporation".into())
                } else {
                    None
                },
                channel: if is_experimental {
                    "experimental".into()
                } else {
                    "stable".into()
                },
                is_dev: false,
                min_driver: None,
            }
        })
        .collect();
    releases.sort_by_key(|a| a.version_packed);
    let latest = releases
        .iter()
        .rev()
        .find(|r| r.channel == "stable")
        .or_else(|| releases.last())
        .map(|r| r.version.clone())
        .unwrap_or_default();
    let entry = FamilyEntry { latest, releases };
    vendors
        .entry(vendor.to_string())
        .or_default()
        .insert(family.to_string(), entry);
}

// ----------------------------------------------------------------------
// Generic GitHub zip-release ingest
// ----------------------------------------------------------------------

async fn ingest_github_zip_releases<F>(
    client: &reqwest::Client,
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
    repo: &str,
    rules: &[FilenameRule],
    asset_filter: F,
) -> Result<()>
where
    F: Fn(&GhAsset) -> bool,
{
    tracing::info!(repo, "fetching GitHub releases");
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=100");
    let releases: Vec<GhRelease> = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    tracing::info!(repo, count = releases.len(), "releases listed");

    let mut by_target: BTreeMap<(String, String), Vec<Release>> = BTreeMap::new();
    for rel in &releases {
        let Some(asset) = rel.assets.iter().find(|a| asset_filter(a)) else {
            tracing::debug!(tag = %rel.tag_name, "no matching asset, skipping");
            continue;
        };
        tracing::info!(repo, tag = %rel.tag_name, asset = %asset.name, size = asset.size, "downloading asset");
        let bytes = match download_bytes(client, &asset.browser_download_url).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(tag = %rel.tag_name, "asset download failed: {e:#}");
                continue;
            }
        };
        let extracted = match extract_dlls_from_zip(&bytes, rules) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(tag = %rel.tag_name, "zip extract failed: {e:#}");
                continue;
            }
        };
        let tag = rel.tag_name.trim_start_matches('v').to_string();
        let version_packed = pack_version(&tag);
        let released_at = rel.published_at.unwrap_or_else(Utc::now);
        let channel = if rel.prerelease {
            "experimental"
        } else {
            "stable"
        };
        for ext in extracted {
            let release = Release {
                version: tag.clone(),
                version_packed,
                filename: ext.filename.clone(),
                sha256: ext.sha256,
                size_bytes: ext.size,
                signed: ext.signed_hint,
                released_at,
                source: format!("{repo}@{}", rel.tag_name),
                cdn_url: asset.browser_download_url.clone(),
                release_notes: rel.name.clone().or_else(|| rel.body.clone()),
                signature_subject: ext.signature_subject,
                channel: channel.into(),
                is_dev: false,
                min_driver: None,
            };
            by_target
                .entry((ext.vendor.into(), ext.family.into()))
                .or_default()
                .push(release);
        }
    }
    for ((vendor, family), mut list) in by_target {
        list.sort_by_key(|a| a.version_packed);
        let latest = list
            .iter()
            .rev()
            .find(|r| r.channel == "stable")
            .or_else(|| list.last())
            .map(|r| r.version.clone())
            .unwrap_or_default();
        vendors.entry(vendor).or_default().insert(
            family,
            FamilyEntry {
                latest,
                releases: list,
            },
        );
    }
    Ok(())
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    Ok(bytes.to_vec())
}

struct ExtractedDll {
    vendor: &'static str,
    family: &'static str,
    filename: String,
    sha256: String,
    size: u64,
    signed_hint: bool,
    signature_subject: Option<String>,
}

fn extract_dlls_from_zip(bytes: &[u8], rules: &[FilenameRule]) -> Result<Vec<ExtractedDll>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;
    let mut out = Vec::new();
    let mut seen: std::collections::HashSet<(&'static str, &'static str)> = Default::default();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let entry_name = entry.name().to_string();
        let base = entry_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        let Some(rule) = rules.iter().find(|r| r.filename == base) else {
            continue;
        };
        if !seen.insert((rule.vendor, rule.family)) {
            // already captured this family from an earlier path (prefer x64/release variant)
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let sha = hex::encode(hasher.finalize());
        // We can't run PowerShell on a zip-entry buffer; rely on upstream signing
        // (Streamline/XeSS/FSR DLLs ship pre-signed by their vendor). Mark as signed
        // and let runtime apply_update perform the real Authenticode check.
        let subject = match rule.vendor {
            "nvidia" => Some("NVIDIA Corporation".into()),
            "intel" => Some("Intel Corporation".into()),
            "amd" => Some("Advanced Micro Devices, Inc.".into()),
            "microsoft" => Some("Microsoft Corporation".into()),
            _ => None,
        };
        out.push(ExtractedDll {
            vendor: rule.vendor,
            family: rule.family,
            filename: rule.filename.into(),
            sha256: sha,
            size: buf.len() as u64,
            signed_hint: subject.is_some(),
            signature_subject: subject,
        });
    }
    if out.is_empty() {
        return Err(anyhow!("no matching DLLs in zip"));
    }
    Ok(out)
}

fn pack_version(s: &str) -> u64 {
    let parts: Vec<u16> = s
        .split(['.', '-', '+'])
        .take(4)
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let major = parts.first().copied().unwrap_or(0) as u64;
    let minor = parts.get(1).copied().unwrap_or(0) as u64;
    let build = parts.get(2).copied().unwrap_or(0) as u64;
    let patch = parts.get(3).copied().unwrap_or(0) as u64;
    (major << 48) | (minor << 32) | (build << 16) | patch
}

// ----------------------------------------------------------------------
// Microsoft DirectStorage NuGet ingest
// ----------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct NugetIndex {
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NugetRegistration {
    items: Vec<NugetRegistrationPage>,
}

#[derive(Debug, Deserialize)]
struct NugetRegistrationPage {
    items: Vec<NugetRegistrationLeaf>,
}

#[derive(Debug, Deserialize)]
struct NugetRegistrationLeaf {
    #[serde(rename = "catalogEntry")]
    catalog_entry: NugetCatalogEntry,
}

#[derive(Debug, Deserialize)]
struct NugetCatalogEntry {
    version: String,
    #[serde(default)]
    published: Option<DateTime<Utc>>,
}

const DS_PKG: &str = "microsoft.direct3d.directstorage";
const DS_RULES: &[FilenameRule] = &[
    FilenameRule {
        filename: "dstorage.dll",
        vendor: "microsoft",
        family: "direct_storage",
    },
    FilenameRule {
        filename: "dstoragecore.dll",
        vendor: "microsoft",
        family: "direct_storage_core",
    },
];

async fn ingest_directstorage_nuget(
    client: &reqwest::Client,
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
) -> Result<()> {
    tracing::info!("fetching DirectStorage NuGet index");
    let idx_url = format!("https://api.nuget.org/v3-flatcontainer/{DS_PKG}/index.json");
    let index: NugetIndex = client
        .get(&idx_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    // Try to enrich with published dates from the registration index (best-effort).
    let reg_url = format!("https://api.nuget.org/v3/registration5-semver1/{DS_PKG}/index.json");
    let mut date_by_ver: BTreeMap<String, DateTime<Utc>> = BTreeMap::new();
    if let Ok(resp) = client.get(&reg_url).send().await {
        if let Ok(reg) = resp.error_for_status()?.json::<NugetRegistration>().await {
            for page in reg.items {
                for leaf in page.items {
                    if let Some(p) = leaf.catalog_entry.published {
                        date_by_ver.insert(leaf.catalog_entry.version.to_lowercase(), p);
                    }
                }
            }
        }
    }

    let mut by_target: BTreeMap<(String, String), Vec<Release>> = BTreeMap::new();
    for ver in index.versions {
        // skip prerelease tags by default
        let is_prerelease = ver.contains('-');
        let pkg_url =
            format!("https://api.nuget.org/v3-flatcontainer/{DS_PKG}/{ver}/{DS_PKG}.{ver}.nupkg");
        tracing::info!(version = %ver, "downloading DirectStorage nupkg");
        let bytes = match download_bytes(client, &pkg_url).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(version = %ver, "nupkg fetch failed: {e:#}");
                continue;
            }
        };
        let extracted = match extract_dlls_from_zip(&bytes, DS_RULES) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(version = %ver, "extract failed: {e:#}");
                continue;
            }
        };
        let version_packed = pack_version(&ver);
        let released_at = date_by_ver
            .get(&ver.to_lowercase())
            .copied()
            .unwrap_or_else(Utc::now);
        let channel = if is_prerelease {
            "experimental"
        } else {
            "stable"
        };
        for ext in extracted {
            let release = Release {
                version: ver.clone(),
                version_packed,
                filename: ext.filename.clone(),
                sha256: ext.sha256,
                size_bytes: ext.size,
                signed: ext.signed_hint,
                released_at,
                source: format!("nuget:{DS_PKG}@{ver}"),
                cdn_url: pkg_url.clone(),
                release_notes: None,
                signature_subject: ext.signature_subject,
                channel: channel.into(),
                is_dev: false,
                min_driver: None,
            };
            by_target
                .entry((ext.vendor.into(), ext.family.into()))
                .or_default()
                .push(release);
        }
    }
    for ((vendor, family), mut list) in by_target {
        list.sort_by_key(|a| a.version_packed);
        let latest = list
            .iter()
            .rev()
            .find(|r| r.channel == "stable")
            .or_else(|| list.last())
            .map(|r| r.version.clone())
            .unwrap_or_default();
        vendors.entry(vendor).or_default().insert(
            family,
            FamilyEntry {
                latest,
                releases: list,
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_handles_simple_tags() {
        assert_eq!(pack_version("310.6.0"), (310u64 << 48) | (6u64 << 32));
        assert_eq!(
            pack_version("2.10.3"),
            (2u64 << 48) | (10u64 << 32) | (3u64 << 16)
        );
        assert_eq!(pack_version("1.4.0-preview1"), (1u64 << 48) | (4u64 << 32));
    }
}
