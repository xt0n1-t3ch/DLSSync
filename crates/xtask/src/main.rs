use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Debug, Deserialize)]
struct CargoManifest {
    workspace: Workspace,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    package: WorkspacePackage,
}

#[derive(Debug, Deserialize)]
struct WorkspacePackage {
    version: String,
}

#[derive(Debug, Deserialize)]
struct ProductRegistry {
    product: ProductIdentity,
    catalog: ProductCatalog,
    links: ProductLinks,
}

#[derive(Debug, Deserialize)]
struct ProductIdentity {
    name: String,
    repository: String,
    manifest_repository: String,
    nexus: String,
    homepage: String,
}

#[derive(Debug, Deserialize)]
struct ProductCatalog {
    canonical_manifest: String,
    signature_suffix: String,
}

#[derive(Debug, Deserialize)]
struct ProductLinks {
    releases: String,
    releases_latest: String,
    issues: String,
    new_issue: String,
    author: String,
    sponsor: String,
    kofi: String,
    anticheat_faq: String,
}

#[derive(Debug, Deserialize)]
struct CompetitiveRegistry {
    as_of: String,
    corrections: String,
    features: Vec<CompetitiveFeature>,
    products: Vec<CompetitiveProduct>,
}

#[derive(Debug, Deserialize)]
struct CompetitiveFeature {
    label: String,
    dlssync: String,
    renderpilot: String,
}

#[derive(Debug, Deserialize)]
struct CompetitiveProduct {
    name: String,
    release: String,
    repository: String,
    sources: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("version") => set_version(&root, args.next().as_deref()),
        Some("check-bindings") => check_bindings(&root),
        Some("generate-bindings") => generate_bindings(&root),
        Some("generate-product") => generate_product(&root),
        Some("check-product") => check_product(&root),
        Some("check-architecture") => check_architecture(&root),
        Some("generate-competitive") => generate_competitive(&root),
        Some("check-competitive") => check_competitive(&root),
        Some("verify-release") => verify_release(&root, args.collect()),
        _ => Err(
            "usage: cargo xtask <version 1.2.3|generate-bindings|check-bindings|generate-product|check-product|check-architecture|generate-competitive|check-competitive|verify-release --channel standard|nexus|portable>"
                .into(),
        ),
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve workspace root".into())
}

fn workspace_version(root: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(root.join("Cargo.toml")).map_err(display)?;
    toml::from_str::<CargoManifest>(&raw)
        .map(|manifest| manifest.workspace.package.version)
        .map_err(display)
}

fn set_version(root: &Path, version: Option<&str>) -> Result<(), String> {
    let version = version.ok_or_else(|| "version requires a SemVer value".to_string())?;
    validate_semver(version)?;
    replace_version(root.join("Cargo.toml"), "version = \"", version)?;
    for path in [
        root.join("package.json"),
        root.join("frontend/package.json"),
        root.join("src-tauri/tauri.conf.json"),
    ] {
        replace_json_version(path, version)?;
    }
    run_command(root, "cargo", &["check", "--workspace"])?;
    verify_version_surfaces(root, version)
}

fn validate_semver(version: &str) -> Result<(), String> {
    let lanes: Vec<_> = version.split('.').collect();
    if lanes.len() != 3 || lanes.iter().any(|lane| lane.parse::<u64>().is_err()) {
        return Err(format!("invalid release SemVer: {version}"));
    }
    Ok(())
}

fn replace_version(path: PathBuf, prefix: &str, version: &str) -> Result<(), String> {
    let raw = fs::read_to_string(&path).map_err(display)?;
    let start = raw
        .find(prefix)
        .ok_or_else(|| format!("version owner missing in {}", path.display()))?
        + prefix.len();
    let end = raw[start..]
        .find('"')
        .map(|offset| start + offset)
        .ok_or_else(|| format!("version terminator missing in {}", path.display()))?;
    let mut updated = raw;
    updated.replace_range(start..end, version);
    fs::write(path, updated).map_err(display)
}

fn replace_json_version(path: PathBuf, version: &str) -> Result<(), String> {
    let raw = fs::read_to_string(&path).map_err(display)?;
    let old = raw
        .lines()
        .find(|line| line.trim_start().starts_with("\"version\":"))
        .ok_or_else(|| format!("JSON version missing in {}", path.display()))?;
    let indent = old.len() - old.trim_start().len();
    let comma = if old.trim_end().ends_with(',') {
        ","
    } else {
        ""
    };
    let new = format!("{}\"version\": \"{version}\"{comma}", " ".repeat(indent));
    fs::write(&path, raw.replacen(old, &new, 1)).map_err(display)
}

fn verify_version_surfaces(root: &Path, expected: &str) -> Result<(), String> {
    for path in [
        root.join("Cargo.toml"),
        root.join("package.json"),
        root.join("frontend/package.json"),
        root.join("src-tauri/tauri.conf.json"),
    ] {
        let raw = fs::read_to_string(&path).map_err(display)?;
        if !raw.contains(expected) {
            return Err(format!("{} does not contain {expected}", path.display()));
        }
    }
    Ok(())
}

fn generate_bindings(root: &Path) -> Result<(), String> {
    run_command(
        root,
        "cargo",
        &[
            "test",
            "-p",
            "dlssync",
            "export_typescript_bindings",
            "--",
            "--ignored",
        ],
    )
}

fn check_bindings(root: &Path) -> Result<(), String> {
    let path = root.join("frontend/src/generated/bindings.ts");
    let before = fs::read(&path).map_err(|_| {
        format!(
            "{} is missing; run cargo xtask generate-bindings",
            path.display()
        )
    })?;
    generate_bindings(root)?;
    let after = fs::read(&path).map_err(display)?;
    if before != after {
        return Err("generated TypeScript bindings were stale".into());
    }
    Ok(())
}

fn product_registry(root: &Path) -> Result<ProductRegistry, String> {
    let raw = fs::read_to_string(root.join("product.toml")).map_err(display)?;
    toml::from_str(&raw).map_err(display)
}

fn product_typescript(registry: &ProductRegistry) -> Result<String, String> {
    let value = serde_json::json!({
        "name": registry.product.name,
        "repository": registry.product.repository,
        "manifestRepository": registry.product.manifest_repository,
        "nexus": registry.product.nexus,
        "homepage": registry.product.homepage,
        "canonicalManifest": registry.catalog.canonical_manifest,
        "signatureSuffix": registry.catalog.signature_suffix,
        "releases": registry.links.releases,
        "releasesLatest": registry.links.releases_latest,
        "issues": registry.links.issues,
        "newIssue": registry.links.new_issue,
        "author": registry.links.author,
        "sponsor": registry.links.sponsor,
        "kofi": registry.links.kofi,
        "anticheatFaq": registry.links.anticheat_faq,
    });
    let json = serde_json::to_string_pretty(&value).map_err(display)?;
    Ok(format!(
        "// Generated by cargo xtask generate-product. Do not edit.\nexport const PRODUCT = {json} as const;\n"
    ))
}

fn generate_product(root: &Path) -> Result<(), String> {
    let generated = product_typescript(&product_registry(root)?)?;
    fs::write(root.join("frontend/src/generated/product.ts"), generated).map_err(display)
}

fn check_product(root: &Path) -> Result<(), String> {
    let expected = product_typescript(&product_registry(root)?)?;
    let path = root.join("frontend/src/generated/product.ts");
    let actual = fs::read_to_string(&path).map_err(|_| {
        format!(
            "{} is missing; run cargo xtask generate-product",
            path.display()
        )
    })?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{} is stale; run cargo xtask generate-product",
            path.display()
        ))
    }
}

fn check_architecture(root: &Path) -> Result<(), String> {
    let source = root.join("frontend/src");
    let mut violations = Vec::new();
    visit_files(&source, &mut |path| {
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("ts" | "svelte")
        ) {
            return;
        }
        if path
            .components()
            .any(|part| part.as_os_str() == "generated")
        {
            return;
        }
        if let Ok(raw) = fs::read_to_string(path) {
            for (line, text) in raw.lines().enumerate() {
                if text.contains("invoke(") || text.contains("invoke<") {
                    violations.push(format!("{}:{} raw Tauri invoke", path.display(), line + 1));
                }
                if text.contains("transport(\"")
                    || text.contains("transport('")
                    || text.contains("transport(`")
                    || text.contains("invokeCommand(\"")
                    || text.contains("invokeCommand('")
                    || text.contains("invokeCommand(`")
                {
                    violations.push(format!(
                        "{}:{} raw command string literal — call transport(COMMANDS.<name>) from generated/bindings.ts",
                        path.display(),
                        line + 1
                    ));
                }
                if [
                    "github.com/xt0n1-t3ch",
                    "nexusmods.com/site/mods/1922",
                    "DLSSync-Manifest",
                ]
                .iter()
                .any(|contract| text.contains(contract))
                {
                    violations.push(format!(
                        "{}:{} product URL outside generated/product.ts",
                        path.display(),
                        line + 1
                    ));
                }
            }
        }
    })?;
    if !violations.is_empty() {
        return Err(format!(
            "frontend contract ownership violations:\n{}",
            violations.join("\n")
        ));
    }
    Ok(())
}

fn competitive_markdown(root: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(root.join("data/competitive-products.json")).map_err(display)?;
    let registry: CompetitiveRegistry = serde_json::from_str(&raw).map_err(display)?;
    if registry.products.len() < 2 || registry.features.is_empty() {
        return Err("competitive registry must contain products and features".into());
    }
    for product in &registry.products {
        if !product.repository.starts_with("https://")
            || product
                .sources
                .iter()
                .any(|source| !source.starts_with("https://"))
        {
            return Err(format!("{} has a non-HTTPS source", product.name));
        }
    }
    let mut output = format!(
        "<!-- Generated by cargo xtask generate-competitive. -->\n# DLSSync and RenderPilot\n\nObserved `{}`. {}\n\n| Capability | DLSSync {} | RenderPilot {} |\n|---|:---:|:---:|\n",
        registry.as_of,
        registry.corrections,
        registry.products[0].release,
        registry.products[1].release,
    );
    for feature in registry.features {
        output.push_str(&format!(
            "| {} | {} | {} |\n",
            feature.label,
            mark(&feature.dlssync),
            mark(&feature.renderpilot)
        ));
    }
    output.push_str("\n## Sources\n\n");
    for product in registry.products {
        output.push_str(&format!("- [{}]({})", product.name, product.repository));
        for source in product.sources.into_iter().skip(1) {
            output.push_str(&format!(" · [evidence]({source})"));
        }
        output.push('\n');
    }
    Ok(output)
}

fn mark(value: &str) -> &'static str {
    if value == "yes" {
        "Yes"
    } else {
        "No"
    }
}

fn generate_competitive(root: &Path) -> Result<(), String> {
    fs::write(
        root.join("docs/competitive-comparison.md"),
        competitive_markdown(root)?,
    )
    .map_err(display)
}

fn check_competitive(root: &Path) -> Result<(), String> {
    let expected = competitive_markdown(root)?;
    let path = root.join("docs/competitive-comparison.md");
    let actual = fs::read_to_string(&path).map_err(display)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{} is stale", path.display()))
    }
}

fn verify_release(root: &Path, args: Vec<String>) -> Result<(), String> {
    check_product(root)?;
    let channel = args
        .windows(2)
        .find(|pair| pair[0] == "--channel")
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| "verify-release requires --channel".to_string())?;
    if !matches!(channel, "standard" | "nexus" | "portable") {
        return Err(format!("unknown release channel: {channel}"));
    }
    let version = workspace_version(root)?;
    verify_version_surfaces(root, &version)?;
    let product = fs::read_to_string(root.join("product.toml")).map_err(display)?;
    if !product.contains(&format!("[distribution.{channel}]")) {
        return Err(format!("product.toml has no {channel} distribution policy"));
    }
    if channel == "nexus" {
        let cargo = fs::read_to_string(root.join("src-tauri/Cargo.toml")).map_err(display)?;
        if !cargo.contains("nexus =") {
            return Err("Nexus Cargo feature is missing".into());
        }
        let catalog_view =
            fs::read_to_string(root.join("frontend/src/views/Catalog.svelte")).map_err(display)?;
        if catalog_view.contains("void loadCatalog();") {
            return Err("Nexus Catalog mount must not trigger a network refresh".into());
        }
    }
    if channel == "portable" {
        let workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).map_err(display)?;
        if !workflow.contains("portable.flag") || !workflow.contains("under .\\data") {
            return Err(
                "portable archive must carry portable.flag and local-data documentation".into(),
            );
        }
    }
    println!("DLSSync {version} {channel} release contract verified");
    Ok(())
}

fn run_command(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(display)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed with {status}", args.join(" ")))
    }
}

fn visit_files(root: &Path, visit: &mut impl FnMut(&Path)) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(display)? {
        let entry = entry.map_err(display)?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, visit)?;
        } else {
            visit(&path);
        }
    }
    Ok(())
}

fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod product_generation_tests {
    use super::*;

    #[test]
    fn generated_product_contract_contains_every_public_link() {
        let registry: ProductRegistry = toml::from_str(
            r#"
                [product]
                name = "DLSSync"
                repository = "https://example.test/app"
                manifest_repository = "https://example.test/manifest"
                nexus = "https://example.test/nexus"
                homepage = "https://example.test/home"

                [catalog]
                canonical_manifest = "https://example.test/manifest.json"
                signature_suffix = ".sig"

                [links]
                releases = "https://example.test/releases"
                releases_latest = "https://example.test/releases/latest"
                issues = "https://example.test/issues"
                new_issue = "https://example.test/issues/new"
                author = "https://example.test/author"
                sponsor = "https://example.test/sponsor"
                kofi = "https://example.test/kofi"
                anticheat_faq = "https://example.test/anticheat"
            "#,
        )
        .unwrap();

        let generated = product_typescript(&registry).unwrap();

        assert!(generated.starts_with("// Generated by cargo xtask generate-product."));
        assert!(generated.contains("\"repository\": \"https://example.test/app\""));
        assert!(generated.contains("\"canonicalManifest\": \"https://example.test/manifest.json\""));
        assert!(generated.contains("\"anticheatFaq\": \"https://example.test/anticheat\""));
    }
}
