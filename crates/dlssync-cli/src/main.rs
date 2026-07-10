use backup_store::BackupStore;
use clap::{Args, Parser, Subcommand};
use dll_catalog::Catalog;
use dlssync_application::{
    apply_update_plan, build_update_plan_at, plan_items, product_config, resolve_data_root,
    rollback_update_plan, scan_installed_games, scan_path, DistributionPolicy,
};
use dlssync_contracts::{
    CatalogProvenance, CatalogRefreshTrigger, DistributionChannel, InstallMode, JournalFilter,
    OperationActor, OperationKind, OperationRecord, OperationStatus, UpdatePlan,
};
use operation_journal::JournalStore;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "dlssync",
    version,
    about = "Trusted DLL and driver synchronization"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Status,
    Scan(ScanArgs),
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
    Plan(PlanArgs),
    Apply(ApplyArgs),
    Rollback(RollbackArgs),
    Journal {
        #[command(subcommand)]
        command: JournalCommand,
    },
    Doctor,
}

#[derive(Args)]
struct ScanArgs {
    #[arg(long)]
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum CatalogCommand {
    Status,
    Refresh,
}

#[derive(Args)]
struct PlanArgs {
    #[arg(long, conflicts_with = "all")]
    game: Option<String>,
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
struct ApplyArgs {
    #[arg(long)]
    plan: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct RollbackArgs {
    #[arg(long)]
    operation: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Subcommand)]
enum JournalCommand {
    List,
    Export,
}

#[derive(Debug, Clone)]
struct RuntimePaths {
    mode: InstallMode,
    root: PathBuf,
    cache: PathBuf,
    backups: PathBuf,
    plans: PathBuf,
    catalog: PathBuf,
    catalog_metadata: PathBuf,
    journal: PathBuf,
    backups_db: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Message(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalog: {0}")]
    Catalog(#[from] dll_catalog::CatalogError),
    #[error("journal: {0}")]
    Journal(#[from] operation_journal::JournalError),
    #[error("backup: {0}")]
    Backup(#[from] backup_store::BackupError),
    #[error("scan: {0}")]
    Scan(#[from] dlssync_application::ScanUseCaseError),
    #[error("execution: {0}")]
    Execution(#[from] dlssync_application::ExecutionError),
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli).await {
        Ok(value) => {
            print_value(cli.json, &value);
            ExitCode::SUCCESS
        }
        Err(error) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({ "ok": false, "error": error.to_string() })
                );
            } else {
                eprintln!("dlssync: {error}");
            }
            ExitCode::from(1)
        }
    }
}

async fn run(cli: &Cli) -> Result<serde_json::Value, CliError> {
    let paths = runtime_paths()?;
    ensure_paths(&paths)?;
    match &cli.command {
        Command::Status => status(&paths),
        Command::Scan(args) => {
            let games = match &args.path {
                Some(path) => vec![scan_path(path)?],
                None => scan_installed_games()?,
            };
            Ok(serde_json::to_value(games)?)
        }
        Command::Catalog { command } => match command {
            CatalogCommand::Status => catalog_status(&paths),
            CatalogCommand::Refresh => catalog_refresh(&paths).await,
        },
        Command::Plan(args) => create_plan(&paths, args),
        Command::Apply(args) => apply_plan(&paths, args).await,
        Command::Rollback(args) => rollback_plan(&paths, args),
        Command::Journal { command } => journal(&paths, command),
        Command::Doctor => doctor(&paths),
    }
}

fn runtime_paths() -> Result<RuntimePaths, CliError> {
    let config = product_config().map_err(|error| CliError::Message(error.to_string()))?;
    let executable = std::env::current_exe()?;
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or_else(|| CliError::Message("home directory is unavailable".into()))?;
    let override_root = std::env::var_os("DLSSYNC_DATA_DIR").map(PathBuf::from);
    let resolved = resolve_data_root(
        &executable,
        home.join(if cfg!(windows) { "DLSSync" } else { ".dlssync" }),
        override_root,
        &config.distribution.portable.data_marker,
    );
    let cache = resolved.root.join("Cache");
    let backups = resolved.root.join("Backups");
    Ok(RuntimePaths {
        mode: resolved.mode,
        plans: resolved.root.join("Plans"),
        catalog: cache.join("catalog.json"),
        catalog_metadata: cache.join("catalog.metadata.json"),
        journal: cache.join("operations.db"),
        backups_db: backups.join("backups.db"),
        root: resolved.root,
        cache,
        backups,
    })
}

fn ensure_paths(paths: &RuntimePaths) -> Result<(), CliError> {
    for path in [&paths.root, &paths.cache, &paths.backups, &paths.plans] {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

fn status(paths: &RuntimePaths) -> Result<serde_json::Value, CliError> {
    let config = product_config().map_err(|error| CliError::Message(error.to_string()))?;
    let policy = DistributionPolicy::resolve(&config, DistributionChannel::Standard, paths.mode);
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "install_mode": paths.mode,
        "data_root": paths.root,
        "catalog_cached": paths.catalog.is_file(),
        "automatic_catalog_refresh": policy.automatic_catalog_refresh,
        "app_updates": policy.app_updates,
    }))
}

fn catalog_status(paths: &RuntimePaths) -> Result<serde_json::Value, CliError> {
    let catalog = load_catalog(paths)?;
    let provenance = load_provenance(paths).unwrap_or_else(|| provenance(&catalog));
    Ok(serde_json::json!({
        "generated_at": catalog.generated_at,
        "vendors": catalog.vendors.len(),
        "provenance": provenance,
    }))
}

async fn catalog_refresh(paths: &RuntimePaths) -> Result<serde_json::Value, CliError> {
    let config = product_config().map_err(|error| CliError::Message(error.to_string()))?;
    let current = load_catalog(paths)?;
    let client = http_client()?;
    let catalog = Catalog::fetch_verified_with_cache_from(
        &client,
        &paths.catalog,
        &config.catalog.canonical_manifest,
        Some(current.generated_at),
    )
    .await?;
    let provenance = provenance(&catalog);
    write_json_atomic(&paths.catalog_metadata, &provenance)?;
    append_record(
        paths,
        OperationKind::CatalogRefresh,
        OperationStatus::Succeeded,
        "Signed catalog refreshed",
        BTreeMap::from([("generated_at".into(), catalog.generated_at.to_rfc3339())]),
        None,
    )?;
    Ok(serde_json::json!({ "refreshed": true, "provenance": provenance }))
}

fn create_plan(paths: &RuntimePaths, args: &PlanArgs) -> Result<serde_json::Value, CliError> {
    let catalog = load_catalog(paths)?;
    let games = scan_installed_games()?;
    let items = plan_items(&catalog, &games, &paths.backups, args.game.as_deref());
    let plan = build_update_plan_at(&catalog.generated_at.to_rfc3339(), items, &paths.backups);
    write_json_atomic(&plan_path(paths, &plan.id), &plan)?;
    append_record(
        paths,
        OperationKind::Plan,
        OperationStatus::Succeeded,
        "Update plan created",
        BTreeMap::from([
            ("plan_id".into(), plan.id.clone()),
            ("items".into(), plan.items.len().to_string()),
        ]),
        None,
    )?;
    Ok(serde_json::to_value(plan)?)
}

async fn apply_plan(paths: &RuntimePaths, args: &ApplyArgs) -> Result<serde_json::Value, CliError> {
    require_yes(args.yes, "apply")?;
    let plan = read_plan(paths, &args.plan)?;
    let catalog = load_catalog(paths)?;
    let backups = BackupStore::open(paths.backups_db.clone(), paths.backups.clone())?;
    let started = Instant::now();
    let result = apply_update_plan(&catalog, &plan, &http_client()?, &backups).await?;
    let record = append_record(
        paths,
        OperationKind::DllApply,
        OperationStatus::Succeeded,
        "Update plan applied",
        BTreeMap::from([
            ("plan_id".into(), plan.id.clone()),
            ("applied".into(), result.applied.to_string()),
            (
                "duration_ms".into(),
                started.elapsed().as_millis().to_string(),
            ),
        ]),
        Some(plan.id.clone()),
    )?;
    Ok(serde_json::json!({ "operation_id": record.id, "result": result }))
}

fn rollback_plan(paths: &RuntimePaths, args: &RollbackArgs) -> Result<serde_json::Value, CliError> {
    require_yes(args.yes, "rollback")?;
    let journal = JournalStore::open(paths.journal.clone())?;
    let record = journal
        .list(&JournalFilter {
            limit: Some(1_000),
            ..Default::default()
        })?
        .into_iter()
        .find(|record| record.id == args.operation)
        .ok_or_else(|| CliError::Message(format!("operation not found: {}", args.operation)))?;
    let plan_id = record
        .details
        .get("plan_id")
        .ok_or_else(|| CliError::Message("operation has no linked plan".into()))?;
    let plan = read_plan(paths, plan_id)?;
    let result = rollback_update_plan(&plan)?;
    append_record(
        paths,
        OperationKind::Rollback,
        OperationStatus::Succeeded,
        "Update plan rolled back",
        BTreeMap::from([
            ("plan_id".into(), plan.id),
            ("restored".into(), result.restored.to_string()),
        ]),
        None,
    )?;
    Ok(serde_json::to_value(result)?)
}

fn journal(paths: &RuntimePaths, command: &JournalCommand) -> Result<serde_json::Value, CliError> {
    let journal = JournalStore::open(paths.journal.clone())?;
    match command {
        JournalCommand::List => Ok(serde_json::to_value(
            journal.list(&JournalFilter::default())?,
        )?),
        JournalCommand::Export => Ok(serde_json::from_str(
            &journal.export_redacted_json(&JournalFilter::default())?,
        )?),
    }
}

fn doctor(paths: &RuntimePaths) -> Result<serde_json::Value, CliError> {
    let catalog = load_catalog(paths);
    let journal = JournalStore::open(paths.journal.clone());
    let writable = tempfile::NamedTempFile::new_in(&paths.cache).is_ok();
    let checks = serde_json::json!({
        "data_root_writable": writable,
        "catalog_signature_valid": catalog.is_ok(),
        "journal_available": journal.is_ok(),
        "portable_boundary": paths.mode != InstallMode::Portable
            || std::env::var_os("DLSSYNC_DATA_DIR").is_some()
            || paths.root.ends_with("data"),
    });
    let healthy = checks
        .as_object()
        .is_some_and(|values| values.values().all(|value| value == true));
    Ok(serde_json::json!({ "healthy": healthy, "checks": checks }))
}

fn load_catalog(paths: &RuntimePaths) -> Result<Catalog, CliError> {
    if paths.catalog.is_file() {
        if let Some(catalog) = dll_catalog::load_verified_cache(&paths.catalog) {
            return Ok(catalog);
        }
    }
    Ok(dll_catalog::embedded_fallback_catalog()?)
}

fn provenance(catalog: &Catalog) -> CatalogProvenance {
    let config = product_config().expect("embedded product config");
    CatalogProvenance {
        manifest_url: config.catalog.canonical_manifest,
        manifest_repository: config.product.manifest_repository,
        generated_at: catalog.generated_at.to_rfc3339(),
        checked_at: chrono::Utc::now().to_rfc3339(),
        signature_verified: true,
        public_key_fingerprint: dll_catalog::manifest_public_key_fingerprint(),
        source_commit: None,
        trigger: CatalogRefreshTrigger::ManualUser,
    }
}

fn load_provenance(paths: &RuntimePaths) -> Option<CatalogProvenance> {
    serde_json::from_slice(&std::fs::read(&paths.catalog_metadata).ok()?).ok()
}

fn read_plan(paths: &RuntimePaths, id: &str) -> Result<UpdatePlan, CliError> {
    if id.contains(['/', '\\']) || id.contains("..") {
        return Err(CliError::Message("invalid plan id".into()));
    }
    Ok(serde_json::from_slice(&std::fs::read(plan_path(
        paths, id,
    ))?)?)
}

fn plan_path(paths: &RuntimePaths, id: &str) -> PathBuf {
    paths.plans.join(format!("{id}.json"))
}

fn require_yes(yes: bool, operation: &str) -> Result<(), CliError> {
    yes.then_some(())
        .ok_or_else(|| CliError::Message(format!("{operation} requires --yes")))
}

fn append_record(
    paths: &RuntimePaths,
    kind: OperationKind,
    status: OperationStatus,
    summary: &str,
    details: BTreeMap<String, String>,
    backup_id: Option<String>,
) -> Result<OperationRecord, CliError> {
    let record = OperationRecord {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        actor: OperationActor::Cli,
        kind,
        status,
        target: None,
        summary: summary.into(),
        details,
        duration_ms: None,
        backup_id,
        error: None,
    };
    JournalStore::open(paths.journal.clone())?.append(&record)?;
    Ok(record)
}

fn http_client() -> Result<reqwest::Client, CliError> {
    reqwest::Client::builder()
        .user_agent(concat!("DLSSync-CLI/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| CliError::Message(error.to_string()))
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), CliError> {
    let parent = path
        .parent()
        .ok_or_else(|| CliError::Message("output path has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut staged, value)?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn print_value(json: bool, value: &serde_json::Value) {
    if json {
        println!("{}", serde_json::json!({ "ok": true, "data": value }));
    } else if let Some(text) = value.as_str() {
        println!("{text}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(value).expect("JSON value")
        );
    }
}
