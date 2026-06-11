import { spawnSync } from "node:child_process";
import fs from "node:fs";
import {
  appBinaryPath,
  buildArgs,
  buildCommand,
  buildTimeoutMs,
  killScriptPath,
  repoRoot,
} from "./config";

const SKIP_BUILD_ENV = "DLSS_E2E_SKIP_BUILD";

export default function globalSetup(): void {
  spawnSync(process.execPath, [killScriptPath], { stdio: "ignore" });

  if (process.env[SKIP_BUILD_ENV] === "1" && fs.existsSync(appBinaryPath)) {
    return;
  }

  const result = spawnSync(buildCommand, buildArgs, {
    cwd: repoRoot,
    stdio: "inherit",
    timeout: buildTimeoutMs,
  });

  if (result.status !== 0) {
    throw new Error(
      `debug build failed (${buildCommand} ${buildArgs.join(" ")}) with status ${result.status}; signal ${result.signal}`,
    );
  }

  if (!fs.existsSync(appBinaryPath)) {
    throw new Error(`debug build completed but binary is missing at ${appBinaryPath}`);
  }
}
