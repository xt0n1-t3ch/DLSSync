#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(scriptPath), "..");

function normalize(candidate) {
  return path.resolve(candidate).toLowerCase();
}

const allowedExecutablePaths = new Set(
  [
    path.join(repoRoot, "target", "debug", "dlssync.exe"),
    path.join(repoRoot, "target", "release", "dlssync.exe"),
  ].map(normalize),
);

function listWindowsDlssyncProcesses() {
  const query = [
    "$ErrorActionPreference='SilentlyContinue'",
    "$p=Get-CimInstance Win32_Process -Filter \"Name = 'dlssync.exe'\"",
    "$p|Select-Object ProcessId,ExecutablePath|ConvertTo-Json -Compress",
  ].join(";");
  const raw = execFileSync("powershell.exe", ["-NoProfile", "-Command", query], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  }).trim();
  if (!raw) return [];
  const parsed = JSON.parse(raw);
  return Array.isArray(parsed) ? parsed : [parsed];
}

function killAllowedWindowsProcesses() {
  let killed = 0;
  for (const proc of listWindowsDlssyncProcesses()) {
    const executablePath = proc?.ExecutablePath;
    const pid = Number(proc?.ProcessId);
    if (!executablePath || !Number.isInteger(pid) || pid <= 0) continue;
    if (!allowedExecutablePaths.has(normalize(executablePath))) continue;
    try {
      process.kill(pid);
      killed += 1;
    } catch {}
  }
  return killed;
}

function killAllowedUnixProcesses() {
  return 0;
}

const killed = process.platform === "win32" ? killAllowedWindowsProcesses() : killAllowedUnixProcesses();

if (killed > 0) {
  await new Promise((resolve) => setTimeout(resolve, 700));
  console.log(`[kill-dlssync] closed ${killed} DLSSync debug/build process(es) from this repo`);
} else {
  console.log("[kill-dlssync] no DLSSync debug/build process from this repo");
}
