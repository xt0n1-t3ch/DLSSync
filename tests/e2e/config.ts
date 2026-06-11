import path from "node:path";
import fs from "node:fs";

const here = __dirname;
export const repoRoot = path.resolve(here, "..", "..");

export const cdpHost = "127.0.0.1";
export const cdpPort = 9333;
export const cdpBaseUrl = `http://${cdpHost}:${cdpPort}`;
export const cdpVersionEndpoint = `${cdpBaseUrl}/json/version`;

export const appBinaryPath = path.join(repoRoot, "target", "debug", "dlssync.exe");
export const killScriptPath = path.join(repoRoot, "scripts", "kill-dlssync.mjs");

function resolvePnpmCliPath(): string {
  const npmExecPath = process.env.npm_execpath;
  if (npmExecPath && fs.existsSync(npmExecPath)) return npmExecPath;

  const bundledPnpm = path.join(
    path.dirname(process.execPath),
    "node_modules",
    "pnpm",
    "bin",
    "pnpm.cjs",
  );
  if (fs.existsSync(bundledPnpm)) return bundledPnpm;

  throw new Error("pnpm CLI entrypoint not found; run e2e through pnpm");
}

export const buildCommand = process.execPath;
export const buildArgs = [resolvePnpmCliPath(), "tauri", "build", "--debug", "--no-bundle"];
export const buildTimeoutMs = 1_500_000;

export const appReadyTimeoutMs = 30_000;
export const appReadyPollIntervalMs = 400;

export const systemScanTimeoutMs = 60_000;

export const reportDir = path.join(here, ".report");
export const resultsDir = path.join(here, ".results");

function readPackageVersion(): string {
  const pkgPath = path.join(repoRoot, "package.json");
  const raw = JSON.parse(fs.readFileSync(pkgPath, "utf8")) as { version?: string };
  if (!raw.version) throw new Error(`no version field in ${pkgPath}`);
  return raw.version;
}

export const appVersion = readPackageVersion();

export const benignConsolePatterns: RegExp[] = [
  /Access to image at .* has been blocked by CORS policy/i,
  /Failed to load resource: net::ERR_FAILED/i,
  /steamstatic\.com/i,
  /steamgriddb\.com/i,
];

export function isBenignConsoleNoise(text: string): boolean {
  return benignConsolePatterns.some((pattern) => pattern.test(text));
}
