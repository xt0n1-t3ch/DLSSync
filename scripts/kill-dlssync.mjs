#!/usr/bin/env node
import { execSync } from "node:child_process";

function run(cmd) {
  try {
    execSync(cmd, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

const killed =
  process.platform === "win32"
    ? run("taskkill /IM dlssync.exe /F /T")
    : run("pkill -x dlssync");

if (killed) {
  await new Promise((resolve) => setTimeout(resolve, 700));
  console.log("[kill-dlssync] closed running DLSSync instance(s)");
} else {
  console.log("[kill-dlssync] no running DLSSync instance");
}
