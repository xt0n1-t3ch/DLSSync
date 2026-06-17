import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const generatedConfigPath = path.join(root, "target", "nexus", "tauri.conf.json");
const generatedCapabilityPath = path.join(root, "target", "nexus", "default.capability.json");
const frontendEnvPath = path.join(root, "frontend", ".env.nexus");
const libPath = path.join(root, "src-tauri", "src", "lib.rs");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const configText = await readFile(generatedConfigPath, "utf8");
const config = JSON.parse(configText);
assert(config.plugins?.updater?.active === false, "Nexus config must disable plugins.updater.active");
assert(Array.isArray(config.plugins.updater.endpoints) && config.plugins.updater.endpoints.length === 0, "Nexus config must remove updater endpoints");
assert(!configText.includes("latest.json"), "Nexus config must not contain latest.json");

const capabilityText = await readFile(generatedCapabilityPath, "utf8");
const capability = JSON.parse(capabilityText);
assert(!capability.permissions.includes("updater:default"), "Nexus capability must remove updater:default");

const frontendEnv = await readFile(frontendEnvPath, "utf8");
assert(frontendEnv.includes("VITE_DLSSYNC_DISTRIBUTION=nexus"), "frontend/.env.nexus must set the nexus distribution");

const lib = await readFile(libPath, "utf8");
assert(lib.includes('#[cfg(not(feature = "nexus"))]'), "Tauri updater plugin must be cfg-gated off for the nexus feature");

console.log("Nexus build strip checks passed");
