//! Builds DLSSync `manifest.json` from authoritative upstream sources.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use dll_catalog::{normalize_name, AntiCheatEntry, AntiCheatIndex, Catalog, FamilyEntry, Release};
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
        default_value = "dlss_swapper,streamline,xess,fsr,reflex,directstorage,anticheat"
    )]
    sources: Vec<String>,
    /// Emit only the distilled anti-cheat snapshot (for the binary's embedded dataset) to this path.
    #[arg(long)]
    emit_anticheat_snapshot: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SwapperManifest {
    #[serde(default)]
    dlss: Vec<SwapperEntry>,
    #[serde(default)]
    dlss_d: Vec<SwapperEntry>,
    #[serde(default)]
    dlss_g: Vec<SwapperEntry>,
    #[serde(default)]
    fsr_31_dx12: Vec<SwapperEntry>,
    #[serde(default)]
    fsr_31_vk: Vec<SwapperEntry>,
    #[serde(default)]
    xess: Vec<SwapperEntry>,
    #[serde(default)]
    xess_dx11: Vec<SwapperEntry>,
    #[serde(default)]
    xess_fg: Vec<SwapperEntry>,
    #[serde(default)]
    xell: Vec<SwapperEntry>,
}

#[derive(Debug, Deserialize)]
struct SwapperEntry {
    version: String,
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

struct FilenameRule {
    /// canonical lowercase DLL filename present in the zip
    filename: &'static str,
    /// (vendor, family) destination
    vendor: &'static str,
    family: &'static str,
}

const STREAMLINE_RULES: &[FilenameRule] = &[
    FilenameRule {
        filename: "sl.dlss.dll",
        vendor: "nvidia",
        family: "sl_dlss_sr",
    },
    FilenameRule {
        filename: "sl.dlss_g.dll",
        vendor: "nvidia",
        family: "sl_dlss_fg",
    },
    FilenameRule {
        filename: "sl.dlss_d.dll",
        vendor: "nvidia",
        family: "sl_dlss_rr",
    },
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

    if let Some(path) = cli.emit_anticheat_snapshot.as_ref() {
        let index = ingest_anticheat(&client).await?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec(&index)?)?;
        tracing::info!(
            path = %path.display(),
            by_appid = index.by_appid.len(),
            by_name = index.by_name.len(),
            "wrote anti-cheat snapshot"
        );
        return Ok(());
    }

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

    let anticheat = if cli.sources.iter().any(|s| s == "anticheat") && !cli.dry_run {
        match ingest_anticheat(&client).await {
            Ok(index) => Some(index),
            Err(e) => {
                tracing::error!("anticheat ingest failed: {e:#}");
                None
            }
        }
    } else {
        None
    };

    let catalog = Catalog {
        schema_version: 2,
        generated_at: Utc::now(),
        vendors,
        incompatible_games: vec![],
        anticheat,
        anti_cheat_binaries: vec![],
    };

    if cli.dry_run {
        println!("{}", serde_json::to_string_pretty(&catalog)?);
        return Ok(());
    }
    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(&catalog)?;
    std::fs::write(&cli.out, &body)?;
    let total: usize = catalog
        .vendors
        .values()
        .flat_map(|v| v.values())
        .map(|f| f.releases.len())
        .sum();
    tracing::info!(path = %cli.out.display(), releases = total, "wrote manifest");

    match std::env::var(SIGNING_KEY_ENV) {
        Ok(key_hex) => {
            let sig_path = sign_manifest(&cli.out, &body, key_hex.trim())?;
            tracing::info!(path = %sig_path.display(), "wrote detached Ed25519 manifest signature");
        }
        Err(_) => {
            tracing::warn!(
                env = SIGNING_KEY_ENV,
                "manifest written UNSIGNED — set the signing key env to emit manifest.json.sig (release builds enforce signatures)"
            );
        }
    }
    Ok(())
}

/// Env var holding the 32-byte Ed25519 signing seed (hex) used to sign the
/// generated manifest. Kept out of the repo; provisioned at manifest-build time.
const SIGNING_KEY_ENV: &str = "DLSSYNC_MANIFEST_SIGNING_KEY";

/// Sign the exact manifest bytes with the Ed25519 seed and write a detached
/// hex-encoded signature next to the manifest (`<out>.sig`), matching the suffix
/// and format that `dll-catalog` verifies against the baked-in public key.
fn sign_manifest(out: &std::path::Path, body: &[u8], key_hex: &str) -> Result<std::path::PathBuf> {
    let sig_hex = sign_bytes(key_hex, body)?;
    let mut sig_os = out.as_os_str().to_owned();
    sig_os.push(".sig");
    let sig_path = std::path::PathBuf::from(sig_os);
    std::fs::write(&sig_path, sig_hex)?;
    Ok(sig_path)
}

/// Produce the hex-encoded 64-byte detached Ed25519 signature over `body`.
fn sign_bytes(key_hex: &str, body: &[u8]) -> Result<String> {
    use ed25519_dalek::{Signer, SigningKey};
    let seed = hex::decode(key_hex).context("DLSSYNC_MANIFEST_SIGNING_KEY is not valid hex")?;
    let seed: [u8; 32] = seed
        .as_slice()
        .try_into()
        .context("DLSSYNC_MANIFEST_SIGNING_KEY must be a 32-byte (64 hex char) Ed25519 seed")?;
    Ok(hex::encode(
        SigningKey::from_bytes(&seed).sign(body).to_bytes(),
    ))
}

#[cfg(test)]
mod signing_tests {
    use super::sign_bytes;
    use ed25519_dalek::{SigningKey, Verifier};

    #[test]
    fn sign_bytes_roundtrips_against_its_public_key() {
        let seed = [9u8; 32];
        let key_hex = hex::encode(seed);
        let body = br#"{"schema_version":2}"#;
        let sig_hex = sign_bytes(&key_hex, body).unwrap();
        let sig_bytes: [u8; 64] = hex::decode(&sig_hex)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();
        let vk = SigningKey::from_bytes(&seed).verifying_key();
        assert!(vk
            .verify(body, &ed25519_dalek::Signature::from_bytes(&sig_bytes))
            .is_ok());
        assert!(vk
            .verify(
                br#"{"schema_version":3}"#,
                &ed25519_dalek::Signature::from_bytes(&sig_bytes)
            )
            .is_err());
    }

    #[test]
    fn sign_bytes_rejects_malformed_seed() {
        assert!(sign_bytes("not-hex", b"x").is_err());
        assert!(sign_bytes(&"aa".repeat(31), b"x").is_err());
    }
}

const ANTICHEAT_DATASET: &str =
    "https://raw.githubusercontent.com/AreWeAntiCheatYet/AreWeAntiCheatYet/master/games.json";
const PCGW_API: &str = "https://www.pcgamingwiki.com/w/api.php";
const CARGO_PAGE: usize = 500;

/// Tokens that are not real protection names — filtered out of PCGamingWiki list
/// fields (Middleware.Anticheat, Availability.Uses_DRM).
const NOISE_TOKENS: &[&str] = &["none", "false", "true", "unknown", "n/a", "yes", "no"];

/// Anti-tamper / heavy-DRM markers worth flagging from the broad Uses_DRM list
/// (which also carries store launchers we ignore here).
const ANTI_TAMPER_MARKERS: &[&str] = &[
    "denuvo",
    "arxan",
    "vmprotect",
    "themida",
    "securom",
    "safedisc",
    "starforce",
];

/// AreWeAntiCheatYet game record — used only for the Linux/Wine `status` overlay
/// (matched by normalized name); PCGamingWiki supplies the protection lists.
#[derive(Debug, Deserialize)]
struct AwacGame {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: Option<String>,
}

/// PCGamingWiki Steam_AppID columns hold a comma list (base game + DLC). The
/// base game's id is the first entry.
fn first_appid(raw: &str) -> Option<u32> {
    raw.split(',').next().and_then(|t| t.trim().parse().ok())
}

/// Split a PCGW list field, trim, dedupe (case-insensitive), drop noise tokens.
fn clean_tokens(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tok in raw.split(',') {
        let t = tok.trim();
        if t.is_empty() || NOISE_TOKENS.contains(&t.to_ascii_lowercase().as_str()) {
            continue;
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct CargoResp {
    #[serde(default)]
    cargoquery: Vec<CargoRow>,
}

#[derive(Debug, Deserialize)]
struct CargoRow {
    title: serde_json::Value,
}

/// Run a Cargo query across all result pages, returning the flattened `title`
/// objects. `params` excludes `limit`/`offset` (added per page).
async fn cargo_query_all(
    client: &reqwest::Client,
    params: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>> {
    let mut rows = Vec::new();
    let mut offset = 0usize;
    loop {
        let offset_str = offset.to_string();
        let limit_str = CARGO_PAGE.to_string();
        let mut q: Vec<(&str, &str)> = vec![
            ("action", "cargoquery"),
            ("format", "json"),
            ("limit", &limit_str),
            ("offset", &offset_str),
        ];
        q.extend_from_slice(params);
        let resp: CargoResp = client
            .get(PCGW_API)
            .query(&q)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let n = resp.cargoquery.len();
        rows.extend(resp.cargoquery.into_iter().map(|r| r.title));
        if n < CARGO_PAGE {
            break;
        }
        offset += CARGO_PAGE;
    }
    Ok(rows)
}

/// One game's distilled protections, keyed by normalized name during the merge.
#[derive(Default)]
struct GameProtections {
    appid: Option<u32>,
    page: String,
    anticheats: Vec<String>,
    anti_tamper: Vec<String>,
    status: Option<String>,
}

fn merge_list(into: &mut Vec<String>, more: Vec<String>) {
    for m in more {
        if !into.iter().any(|x| x.eq_ignore_ascii_case(&m)) {
            into.push(m);
        }
    }
}

/// Build the anti-cheat / anti-tamper index from PCGamingWiki (broad coverage of
/// all games, keyed by Steam appid + normalized name), with an AreWeAntiCheatYet
/// overlay for Linux/Wine status. PCGW Middleware.Anticheat → `anticheats`;
/// Availability.Uses_DRM Denuvo/Arxan/etc → `anti_tamper`.
async fn ingest_anticheat(client: &reqwest::Client) -> Result<AntiCheatIndex> {
    use std::collections::BTreeMap;
    let mut games: BTreeMap<String, GameProtections> = BTreeMap::new();

    let upsert =
        |games: &mut BTreeMap<String, GameProtections>, page: &str, appid: Option<u32>| -> bool {
            let key = normalize_name(page);
            if key.is_empty() {
                return false;
            }
            let e = games.entry(key).or_default();
            if e.page.is_empty() {
                e.page = page.to_string();
            }
            if e.appid.is_none() {
                e.appid = appid;
            }
            true
        };

    let ac_rows = cargo_query_all(
        client,
        &[
            ("tables", "Middleware,Infobox_game"),
            (
                "fields",
                "Infobox_game._pageName=Page,Middleware.Anticheat=AC,Infobox_game.Steam_AppID=AppID",
            ),
            ("join_on", "Middleware._pageID=Infobox_game._pageID"),
            ("where", "Middleware.Anticheat HOLDS LIKE \"%\""),
        ],
    )
    .await
    .context("PCGW anticheat query")?;
    for t in &ac_rows {
        let page = t.get("Page").and_then(|v| v.as_str()).unwrap_or("");
        let ac = clean_tokens(t.get("AC").and_then(|v| v.as_str()).unwrap_or(""));
        let appid = t
            .get("AppID")
            .and_then(|v| v.as_str())
            .and_then(first_appid);
        if ac.is_empty() || !upsert(&mut games, page, appid) {
            continue;
        }
        let key = normalize_name(page);
        merge_list(&mut games.get_mut(&key).unwrap().anticheats, ac);
    }

    let drm_rows = cargo_query_all(
        client,
        &[
            ("tables", "Availability,Infobox_game"),
            (
                "fields",
                "Infobox_game._pageName=Page,Availability.Uses_DRM=DRM,Infobox_game.Steam_AppID=AppID",
            ),
            ("join_on", "Availability._pageID=Infobox_game._pageID"),
            (
                "where",
                "Availability.Uses_DRM HOLDS LIKE \"%Denuvo%\" OR Availability.Uses_DRM HOLDS LIKE \"%Arxan%\"",
            ),
        ],
    )
    .await
    .context("PCGW anti-tamper query")?;
    for t in &drm_rows {
        let page = t.get("Page").and_then(|v| v.as_str()).unwrap_or("");
        let appid = t
            .get("AppID")
            .and_then(|v| v.as_str())
            .and_then(first_appid);
        let tamper: Vec<String> = clean_tokens(t.get("DRM").and_then(|v| v.as_str()).unwrap_or(""))
            .into_iter()
            .filter(|d| {
                let low = d.to_ascii_lowercase();
                ANTI_TAMPER_MARKERS.iter().any(|m| low.contains(m))
            })
            .collect();
        if tamper.is_empty() || !upsert(&mut games, page, appid) {
            continue;
        }
        let key = normalize_name(page);
        merge_list(&mut games.get_mut(&key).unwrap().anti_tamper, tamper);
    }

    if let Ok(body) = client
        .get(ANTICHEAT_DATASET)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        if let Ok(text) = body.text().await {
            if let Ok(awac) = serde_json::from_str::<Vec<AwacGame>>(&text) {
                for g in awac {
                    let Some(status) = g.status else { continue };
                    let key = normalize_name(&g.name);
                    if let Some(e) = games.get_mut(&key) {
                        e.status.get_or_insert(status);
                    }
                }
            }
        }
    }

    let mut index = AntiCheatIndex::default();
    for (key, g) in games {
        if g.anticheats.is_empty() && g.anti_tamper.is_empty() {
            continue;
        }
        let entry = AntiCheatEntry {
            anticheats: g.anticheats,
            anti_tamper: g.anti_tamper,
            status: g.status,
        };
        index.by_name.insert(key, entry.clone());
        if let Some(appid) = g.appid {
            index.by_appid.insert(appid, entry);
        }
    }
    tracing::info!(
        anticheat_rows = ac_rows.len(),
        tamper_rows = drm_rows.len(),
        by_appid = index.by_appid.len(),
        by_name = index.by_name.len(),
        "distilled PCGamingWiki protection index"
    );
    Ok(index)
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
    merge_swapper(
        vendors,
        "amd",
        "fsr_upscaler",
        "amd_fidelityfx_dx12.dll",
        &manifest.fsr_31_dx12,
    );
    merge_swapper(
        vendors,
        "amd",
        "fsr_upscaler_vk",
        "amd_fidelityfx_vk.dll",
        &manifest.fsr_31_vk,
    );
    merge_swapper(vendors, "intel", "xess_sr", "libxess.dll", &manifest.xess);
    merge_swapper(
        vendors,
        "intel",
        "xess_sr_dx11",
        "libxess_dx11.dll",
        &manifest.xess_dx11,
    );
    merge_swapper(
        vendors,
        "intel",
        "xess_fg",
        "libxess_fg.dll",
        &manifest.xess_fg,
    );
    merge_swapper(vendors, "intel", "xell", "libxell.dll", &manifest.xell);
    Ok(())
}

fn vendor_subject(vendor: &str) -> &'static str {
    match vendor {
        "amd" => "Advanced Micro Devices, Inc.",
        "intel" => "Intel Corporation",
        _ => "NVIDIA Corporation",
    }
}

fn merge_swapper(
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
    vendor: &str,
    family: &str,
    filename: &str,
    entries: &[SwapperEntry],
) {
    let releases: Vec<Release> = entries
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
                version_packed: pack_version(&e.version),
                filename: filename.to_string(),
                sha256: e.md5_hash.clone().to_lowercase(),
                hash_algorithm: "md5".to_string(),
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
                    Some(vendor_subject(vendor).to_string())
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
                zip_entry: None,
            }
        })
        .collect();
    upsert_family(vendors, vendor, family, releases);
}

/// Union new releases into a (vendor, family) entry instead of replacing it, so a
/// second upstream source for the same family (e.g. FidelityFX-SDK on top of
/// dlss-swapper FSR) extends the history rather than clobbering it. Releases are
/// deduped by (version, filename) and `latest` is recomputed from the uniformly
/// packed version across the merged set.
fn upsert_family(
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
    vendor: &str,
    family: &str,
    mut new_releases: Vec<Release>,
) {
    let fam = vendors
        .entry(vendor.to_string())
        .or_default()
        .entry(family.to_string())
        .or_insert_with(|| FamilyEntry {
            latest: String::new(),
            releases: Vec::new(),
        });
    fam.releases.append(&mut new_releases);
    fam.releases.sort_by(|a, b| {
        a.version_packed
            .cmp(&b.version_packed)
            .then_with(|| a.filename.cmp(&b.filename))
    });
    fam.releases
        .dedup_by(|a, b| a.version == b.version && a.filename == b.filename);
    fam.latest = fam
        .releases
        .iter()
        .rev()
        .find(|r| r.channel == "stable")
        .or_else(|| fam.releases.last())
        .map(|r| r.version.clone())
        .unwrap_or_default();
}

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
                hash_algorithm: "sha256".to_string(),
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
                zip_entry: Some(ext.zip_entry.clone()),
            };
            by_target
                .entry((ext.vendor.into(), ext.family.into()))
                .or_default()
                .push(release);
        }
    }
    for ((vendor, family), list) in by_target {
        upsert_family(vendors, &vendor, &family, list);
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
    zip_entry: String,
    sha256: String,
    size: u64,
    signed_hint: bool,
    signature_subject: Option<String>,
}

/// Rank a zip entry path for a basename match: the canonical production runtime
/// (`bin/x64/<name>`) outranks any `/development/` or build-artifact copy, so a
/// multi-copy SDK zip (the Streamline feature plugins ship 4 copies) always
/// records and extracts the signed production binary regardless of entry order.
fn production_rank(path_lower: &str) -> u8 {
    if path_lower.contains("/development/") || path_lower.contains("_artifacts/") {
        0
    } else if path_lower.starts_with("bin/x64/") {
        2
    } else {
        1
    }
}

fn extract_dlls_from_zip(bytes: &[u8], rules: &[FilenameRule]) -> Result<Vec<ExtractedDll>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;
    let mut best: std::collections::HashMap<
        (&'static str, &'static str, &'static str),
        (usize, String, u8),
    > = Default::default();
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let entry_name = entry.name().replace('\\', "/");
        let base = entry_name.rsplit('/').next().unwrap_or("").to_lowercase();
        let Some(rule) = rules.iter().find(|r| r.filename == base) else {
            continue;
        };
        let rank = production_rank(&entry_name.to_lowercase());
        let key = (rule.vendor, rule.family, rule.filename);
        if best
            .get(&key)
            .is_none_or(|(_, _, best_rank)| rank > *best_rank)
        {
            best.insert(key, (i, entry_name, rank));
        }
    }
    let mut out = Vec::new();
    for ((vendor, family, filename), (idx, zip_entry, _)) in best {
        let mut entry = zip.by_index(idx)?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let sha = hex::encode(hasher.finalize());
        let subject = match vendor {
            "nvidia" => Some("NVIDIA Corporation".into()),
            "intel" => Some("Intel Corporation".into()),
            "amd" => Some("Advanced Micro Devices, Inc.".into()),
            "microsoft" => Some("Microsoft Corporation".into()),
            _ => None,
        };
        out.push(ExtractedDll {
            vendor,
            family,
            filename: filename.into(),
            zip_entry,
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
        .map(|p| {
            p.parse::<u64>()
                .map(|n| n.min(u16::MAX as u64) as u16)
                .unwrap_or(0)
        })
        .collect();
    let major = parts.first().copied().unwrap_or(0) as u64;
    let minor = parts.get(1).copied().unwrap_or(0) as u64;
    let build = parts.get(2).copied().unwrap_or(0) as u64;
    let patch = parts.get(3).copied().unwrap_or(0) as u64;
    (major << 48) | (minor << 32) | (build << 16) | patch
}

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

/// DirectStorage versions from the NuGet flat-container API. Uses its OWN
/// unauthenticated client because the shared client carries a GitHub bearer for
/// the release sources, and NuGet's Azure backend rejects any request that
/// presents one with HTTP 403.
async fn ingest_directstorage_nuget(
    _shared: &reqwest::Client,
    vendors: &mut BTreeMap<String, BTreeMap<String, FamilyEntry>>,
) -> Result<()> {
    tracing::info!("fetching DirectStorage NuGet index");
    let client = reqwest::Client::builder()
        .user_agent("dlssync-manifest-builder/0.1 (+https://github.com/xt0n1-t3ch/DLSSync)")
        .build()?;
    let idx_url = format!("https://api.nuget.org/v3-flatcontainer/{DS_PKG}/index.json");
    let index: NugetIndex = client
        .get(&idx_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
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
        let is_prerelease = ver.contains('-');
        let pkg_url =
            format!("https://api.nuget.org/v3-flatcontainer/{DS_PKG}/{ver}/{DS_PKG}.{ver}.nupkg");
        tracing::info!(version = %ver, "downloading DirectStorage nupkg");
        let bytes = match download_bytes(&client, &pkg_url).await {
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
                hash_algorithm: "sha256".to_string(),
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
                zip_entry: Some(ext.zip_entry.clone()),
            };
            by_target
                .entry((ext.vendor.into(), ext.family.into()))
                .or_default()
                .push(release);
        }
    }
    for ((vendor, family), list) in by_target {
        upsert_family(vendors, &vendor, &family, list);
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

    #[test]
    fn streamline_rules_source_sl_dlss_plugins_into_their_own_families() {
        for (file, family) in [
            ("sl.dlss.dll", "sl_dlss_sr"),
            ("sl.dlss_g.dll", "sl_dlss_fg"),
            ("sl.dlss_d.dll", "sl_dlss_rr"),
        ] {
            assert!(
                STREAMLINE_RULES
                    .iter()
                    .any(|r| r.filename == file && r.family == family && r.vendor == "nvidia"),
                "{file} must source family {family} (v1.6 Streamline Set Updater) — never the \
                 nvngx 310.x families (that was the v1.5.2 cross-scheme bug)"
            );
        }
    }

    #[test]
    fn production_rank_prefers_bin_x64_over_development_and_artifacts() {
        assert!(
            production_rank("bin/x64/sl.dlss_g.dll")
                > production_rank("bin/x64/development/sl.dlss_g.dll")
        );
        assert!(
            production_rank("bin/x64/sl.dlss_g.dll")
                > production_rank("_artifacts/sl.dlss_g/production_x64/sl.dlss_g.dll")
        );
        assert_eq!(production_rank("bin/x64/development/sl.dlss_g.dll"), 0);
    }

    #[test]
    fn first_appid_takes_base_game_from_comma_list() {
        assert_eq!(first_appid("1245620"), Some(1245620));
        assert_eq!(first_appid("3768760,4707780, 4601250"), Some(3768760));
        assert_eq!(first_appid(" 990080 "), Some(990080));
        assert_eq!(first_appid(""), None);
        assert_eq!(first_appid("not-a-number"), None);
    }

    #[test]
    fn clean_tokens_splits_trims_dedupes_and_drops_noise() {
        assert_eq!(
            clean_tokens("Easy Anti-Cheat, BattlEye , Easy Anti-Cheat"),
            vec!["Easy Anti-Cheat".to_string(), "BattlEye".to_string()]
        );
        assert!(clean_tokens("None, none, false, , Unknown").is_empty());
        assert_eq!(
            clean_tokens("Steam,Ubisoft Connect,Denuvo Anti-Tamper")
                .into_iter()
                .filter(|d| ANTI_TAMPER_MARKERS
                    .iter()
                    .any(|m| d.to_ascii_lowercase().contains(m)))
                .collect::<Vec<_>>(),
            vec!["Denuvo Anti-Tamper".to_string()]
        );
    }

    #[test]
    fn merge_list_unions_case_insensitively() {
        let mut a = vec!["Denuvo Anti-Tamper".to_string()];
        merge_list(&mut a, vec!["denuvo anti-tamper".into(), "Arxan".into()]);
        assert_eq!(
            a,
            vec!["Denuvo Anti-Tamper".to_string(), "Arxan".to_string()]
        );
    }

    #[test]
    fn merge_swapper_maps_fsr_to_amd_family_with_vendor_subject() {
        let entries: Vec<SwapperEntry> = serde_json::from_str(
            r#"[
            {"version":"1.0.0.36208","version_number":1000036208,"md5_hash":"ABC","download_url":"https://x/fsr_a.zip","file_size":100,"is_signature_valid":true},
            {"version":"1.0.1.41314","version_number":1000141314,"md5_hash":"DEF","download_url":"https://x/fsr_b.zip","file_size":200,"is_signature_valid":true}
        ]"#,
        )
        .unwrap();
        let mut vendors: BTreeMap<String, BTreeMap<String, FamilyEntry>> = BTreeMap::new();
        merge_swapper(
            &mut vendors,
            "amd",
            "fsr_upscaler",
            "amd_fidelityfx_dx12.dll",
            &entries,
        );
        let fam = &vendors["amd"]["fsr_upscaler"];
        assert_eq!(fam.releases.len(), 2);
        assert!(fam
            .releases
            .iter()
            .all(|r| r.filename == "amd_fidelityfx_dx12.dll"));
        assert_eq!(
            fam.releases[0].signature_subject.as_deref(),
            Some("Advanced Micro Devices, Inc.")
        );
        assert_eq!(fam.latest, "1.0.1.41314");
    }

    #[test]
    fn vendor_subject_maps_each_vendor() {
        assert_eq!(vendor_subject("amd"), "Advanced Micro Devices, Inc.");
        assert_eq!(vendor_subject("intel"), "Intel Corporation");
        assert_eq!(vendor_subject("nvidia"), "NVIDIA Corporation");
    }

    fn rel(ver: &str, file: &str) -> Release {
        Release {
            version: ver.to_string(),
            version_packed: pack_version(ver),
            filename: file.to_string(),
            sha256: "x".into(),
            hash_algorithm: "sha256".into(),
            size_bytes: 0,
            signed: false,
            released_at: Utc::now(),
            source: "t".into(),
            cdn_url: "https://x/y".into(),
            release_notes: None,
            signature_subject: None,
            channel: "stable".into(),
            is_dev: false,
            min_driver: None,
            zip_entry: None,
        }
    }

    #[test]
    fn upsert_family_unions_sources_without_clobbering_history() {
        let mut vendors: BTreeMap<String, BTreeMap<String, FamilyEntry>> = BTreeMap::new();
        upsert_family(
            &mut vendors,
            "amd",
            "fsr_upscaler",
            vec![
                rel("3.1.0", "amd_fidelityfx_dx12.dll"),
                rel("3.1.1", "amd_fidelityfx_dx12.dll"),
                rel("3.1.2", "amd_fidelityfx_dx12.dll"),
            ],
        );
        upsert_family(
            &mut vendors,
            "amd",
            "fsr_upscaler",
            vec![rel("2.0.0", "amd_fidelityfx_upscaler_dx12.dll")],
        );
        let fam = &vendors["amd"]["fsr_upscaler"];
        assert_eq!(
            fam.releases.len(),
            4,
            "second source must extend, not clobber, the first"
        );
        assert_eq!(
            fam.latest, "3.1.2",
            "uniform packing keeps the genuinely newest version as latest across sources"
        );
    }

    #[test]
    fn upsert_family_dedupes_same_version_and_filename() {
        let mut vendors: BTreeMap<String, BTreeMap<String, FamilyEntry>> = BTreeMap::new();
        upsert_family(&mut vendors, "amd", "fsr_fg", vec![rel("1.1.2", "a.dll")]);
        upsert_family(&mut vendors, "amd", "fsr_fg", vec![rel("1.1.2", "a.dll")]);
        assert_eq!(vendors["amd"]["fsr_fg"].releases.len(), 1);
    }
}
