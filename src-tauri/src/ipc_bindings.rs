#[cfg(test)]
pub fn export_typescript_bindings() {
    use dlssync_contracts::*;
    use specta::Types;
    use specta_typescript::Typescript;
    use std::io::Write as _;

    let types = Types::default()
        .register::<DistributionChannel>()
        .register::<InstallMode>()
        .register::<CatalogRefreshTrigger>()
        .register::<CatalogDelta>()
        .register::<CatalogProvenance>()
        .register::<CatalogRefreshResult>()
        .register::<CatalogStatus>()
        .register::<OperationActor>()
        .register::<OperationKind>()
        .register::<OperationStatus>()
        .register::<OperationRecord>()
        .register::<JournalFilter>()
        .register::<TrustEvidence>()
        .register::<UpdatePlanItem>()
        .register::<UpdatePlan>()
        .register::<ScannedComponent>()
        .register::<ScannedGame>()
        .register::<ApplyPlanResult>()
        .register::<RollbackPlanResult>()
        .register::<ApiError>();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/src/generated/bindings.ts");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("generated bindings directory");
    }
    Typescript::default()
        .export_to(&path, &types, specta_serde::Format)
        .expect("export contract types");
    let mut output = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open generated bindings");
    output
        .write_all(command_registry_ts().as_bytes())
        .expect("append generated command registry + transport");
}

/// Every Tauri command name, read straight from the `generate_handler!` registry
/// in `lib.rs`, so the projected TypeScript owner can never drift from the actual
/// backend surface. Sorted + deduped for a stable generated file.
#[cfg(test)]
fn command_names() -> Vec<String> {
    let lib = include_str!("lib.rs");
    let inner = lib
        .split_once("generate_handler![")
        .and_then(|(_, rest)| rest.split_once("])"))
        .map(|(inner, _)| inner)
        .expect("generate_handler! block present in lib.rs");
    let mut names: Vec<String> = inner
        .split(',')
        .filter_map(|item| item.trim().rsplit("::").next())
        .map(str::trim)
        .filter(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        })
        .map(String::from)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The generated command-name owner (`COMMANDS`) plus the typed transport. Frontend
/// consumers reference `COMMANDS.<name>` instead of hand-written string literals,
/// and `check-architecture` rejects any raw `transport("...")` outside this file.
#[cfg(test)]
fn command_registry_ts() -> String {
    let mut out = String::from("\n\nimport { invoke as tauriInvoke } from \"@tauri-apps/api/core\";\n\nexport const COMMANDS = {\n");
    for name in command_names() {
        out.push_str(&format!("  {name}: \"{name}\",\n"));
    }
    out.push_str("} as const;\n\n");
    out.push_str("export type CommandName = (typeof COMMANDS)[keyof typeof COMMANDS];\n\n");
    out.push_str(
        "export function invokeCommand<T>(command: CommandName, args?: Record<string, unknown>): Promise<T> {\n  return tauriInvoke<T>(command, args);\n}\n",
    );
    out
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "writes frontend/src/generated/bindings.ts"]
    fn export_typescript_bindings() {
        super::export_typescript_bindings();
    }
}
