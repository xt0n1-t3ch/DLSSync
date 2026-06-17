import { mkdir, readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const targetDir = path.join(root, "target", "nexus");
const generatedConfigPath = path.join(targetDir, "tauri.conf.json");
const generatedCapabilityPath = path.join(targetDir, "default.capability.json");
const tauriConfigPath = path.join(root, "src-tauri", "tauri.conf.json");
const capabilityPath = path.join(root, "src-tauri", "capabilities", "default.json");

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

function stripUpdaterCapability(capability) {
  return {
    ...capability,
    permissions: capability.permissions.filter((permission) => permission !== "updater:default"),
  };
}

function nexusConfig(base) {
  const next = structuredClone(base);
  next.build = {
    ...next.build,
    beforeBuildCommand: "pnpm -w run prebuild:kill && pnpm --filter dlssync-frontend build:nexus",
  };
  next.bundle = {
    ...next.bundle,
    shortDescription: "Nexus-compliant DLSSync build with manual app updates",
    longDescription:
      "Nexus-compliant DLSSync build: DLSS, FSR, XeSS, Streamline, DirectStorage, and driver sync remain intact, while application self-update is disabled so future app versions are installed manually from Nexus Mods or GitHub Releases.",
  };
  if (next.plugins?.updater) {
    next.plugins = { ...next.plugins, updater: { active: false, endpoints: [], dialog: false } };
  }
  return next;
}

async function prepare() {
  await mkdir(targetDir, { recursive: true });
  const config = nexusConfig(await readJson(tauriConfigPath));
  const capability = stripUpdaterCapability(await readJson(capabilityPath));
  await writeFile(generatedConfigPath, `${JSON.stringify(config, null, 2)}\n`);
  await writeFile(generatedCapabilityPath, `${JSON.stringify(capability, null, 2)}\n`);
  return { config, capability };
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: "inherit", shell: process.platform === "win32" });
    child.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(" ")} exited ${code}`));
    });
    child.on("error", reject);
  });
}

const prepareOnly = process.argv.includes("--prepare-only");
await prepare();

if (prepareOnly) {
  console.log(`Prepared Nexus build config: ${generatedConfigPath}`);
  console.log(`Prepared Nexus capability: ${generatedCapabilityPath}`);
  process.exit(0);
}

const originalConfig = await readFile(tauriConfigPath, "utf8");
const originalCapability = await readFile(capabilityPath, "utf8");
try {
  await writeFile(tauriConfigPath, await readFile(generatedConfigPath, "utf8"));
  await writeFile(capabilityPath, await readFile(generatedCapabilityPath, "utf8"));
  await run("pnpm", ["exec", "tauri", "build", "--features", "nexus"]);
} finally {
  await writeFile(tauriConfigPath, originalConfig);
  await writeFile(capabilityPath, originalCapability);
}
