import { test as base, chromium, expect } from "@playwright/test";
import type { Browser, BrowserContext, ConsoleMessage, Page } from "@playwright/test";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  appBinaryPath,
  appReadyPollIntervalMs,
  appReadyTimeoutMs,
  cdpBaseUrl,
  cdpVersionEndpoint,
  isBenignConsoleNoise,
  killScriptPath,
  repoRoot,
} from "./config";

export interface ConsoleNoise {
  readonly errors: string[];
  readonly warnings: string[];
  readonly exceptions: string[];
  readonly ignored: string[];
}

interface AppHarness {
  readonly page: Page;
  readonly noise: ConsoleNoise;
}

const delay = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

function killStaleInstances(): void {
  spawnSync(process.execPath, [killScriptPath], { stdio: "ignore" });
}

const FIXTURE_EXE_BYTES = 5 * 1024 * 1024 + 1024;
export const E2E_PROTECTED_GAME_NAME = "Aurora Protocol";

interface FixtureGame {
  readonly name: string;
  readonly dlls: readonly string[];
  readonly antiCheat?: string;
}

const FIXTURE_GAMES: readonly FixtureGame[] = [
  {
    name: E2E_PROTECTED_GAME_NAME,
    dlls: ["nvngx_dlss.dll", "nvngx_dlssg.dll"],
    antiCheat: "EasyAntiCheat_x64.dll",
  },
  { name: "Neon Divide", dlls: ["nvngx_dlss.dll", "dstorage.dll"] },
  { name: "Atlas Frontier", dlls: ["libxess.dll", "libxess_fg.dll"] },
  {
    name: "Midnight Circuit",
    dlls: ["amd_fidelityfx_dx12.dll", "amd_fidelityfx_framegeneration_dx12.dll"],
  },
];

function seedFixtureGame(gamesRoot: string, game: FixtureGame): void {
  const gameDir = join(gamesRoot, game.name);
  mkdirSync(gameDir, { recursive: true });
  writeFileSync(join(gameDir, `${game.name.replaceAll(" ", "")}.exe`), Buffer.alloc(FIXTURE_EXE_BYTES));
  if (game.antiCheat) writeFileSync(join(gameDir, game.antiCheat), "fixture");
  for (const dll of game.dlls) {
    const dllPath = join(gameDir, dll);
    if (existsSync(appBinaryPath)) copyFileSync(appBinaryPath, dllPath);
    else writeFileSync(dllPath, "fixture");
  }
}

/** Hermetic per-worker data root: the debug app honors DLSSYNC_DATA_DIR, so the
 *  suite never reads or mutates the host's real `~/DLSSync` settings, backups,
 *  or notifications. Seeded fictional games (each with a >=5 MB executable)
 *  make every library state deterministic without exposing host libraries or
 *  relying on copyrighted game artwork. */
function seedHermeticDataDir(): string {
  // The marketing capture publishes these surfaces (About "Data & logs", Settings
  // path, game-detail install path, update-plan file rows), so it uses a clean,
  // generic data root — no developer username or e2e temp-dir name leaks into the
  // public gallery. Regular E2E runs stay fully hermetic under the OS temp dir.
  let dataDir: string;
  if (process.env.DLSSYNC_CAPTURE_MARKETING === "1") {
    dataDir = join(process.env.PUBLIC ?? "C:\\Users\\Public", "DLSSync");
    rmSync(dataDir, { recursive: true, force: true });
    mkdirSync(dataDir, { recursive: true });
  } else {
    dataDir = mkdtempSync(join(tmpdir(), "dlssync-e2e-"));
  }
  const gamesRoot = join(dataDir, "FixtureGames");
  for (const game of FIXTURE_GAMES) seedFixtureGame(gamesRoot, game);
  const settingsDir = join(dataDir, "Settings");
  mkdirSync(settingsDir, { recursive: true });
  writeFileSync(
    join(settingsDir, "settings.json"),
    JSON.stringify({ launcher_overrides: { custom: [gamesRoot] } }),
  );
  return dataDir;
}

function spawnApp(dataDir: string): ChildProcess {
  return spawn(appBinaryPath, [], {
    cwd: repoRoot,
    stdio: "ignore",
    windowsHide: false,
    env: {
      ...process.env,
      DLSSYNC_DATA_DIR: dataDir,
      DLSSYNC_E2E: "1",
      DLSSYNC_E2E_GPU_FIXTURE: "1",
      WEBVIEW2_USER_DATA_FOLDER: join(dataDir, "WebView2"),
    },
  });
}

async function waitForCdp(): Promise<void> {
  const deadline = Date.now() + appReadyTimeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(cdpVersionEndpoint);
      if (res.ok) return;
    } catch (err) {
      lastError = err;
    }
    await delay(appReadyPollIntervalMs);
  }
  throw new Error(
    `app CDP endpoint ${cdpVersionEndpoint} never became ready within ${appReadyTimeoutMs}ms (binary: ${appBinaryPath}; last error: ${String(lastError)})`,
  );
}

async function findAppPage(context: BrowserContext): Promise<Page> {
  const deadline = Date.now() + appReadyTimeoutMs;
  while (Date.now() < deadline) {
    const page = context.pages().find((p) => !p.url().startsWith("devtools://"));
    if (page) return page;
    const opened = await context
      .waitForEvent("page", { timeout: appReadyPollIntervalMs })
      .catch(() => undefined);
    if (opened && !opened.url().startsWith("devtools://")) return opened;
  }
  throw new Error("connected over CDP but no app page target was found");
}

function attachNoiseCollector(page: Page): ConsoleNoise {
  const errors: string[] = [];
  const warnings: string[] = [];
  const exceptions: string[] = [];
  const ignored: string[] = [];
  page.on("console", (msg: ConsoleMessage) => {
    const text = msg.text();
    if (msg.type() === "error") {
      if (isBenignConsoleNoise(text)) ignored.push(text);
      else errors.push(text);
    } else if (msg.type() === "warning") {
      warnings.push(text);
    }
  });
  page.on("pageerror", (err: Error) => {
    exceptions.push(err.message);
  });
  return { errors, warnings, exceptions, ignored };
}

export const test = base.extend<{ consoleGuard: void }, { app: AppHarness }>({
  /** Per-spec console gate: asserts no NEW errors/exceptions landed during each
   *  test, so a worker restart (retries) can never discard accumulated noise the
   *  way a single end-of-run gate would. zz-console-clean stays as the
   *  whole-run catch-all for noise between specs. */
  consoleGuard: [
    async ({ app }, use, testInfo) => {
      const before = {
        errors: app.noise.errors.length,
        exceptions: app.noise.exceptions.length,
      };
      await use();
      const newErrors = app.noise.errors.slice(before.errors);
      const newExceptions = app.noise.exceptions.slice(before.exceptions);
      expect(
        newErrors,
        `console errors during "${testInfo.title}": ${JSON.stringify(newErrors.slice(0, 5))}`,
      ).toHaveLength(0);
      expect(
        newExceptions,
        `exceptions during "${testInfo.title}": ${JSON.stringify(newExceptions.slice(0, 5))}`,
      ).toHaveLength(0);
    },
    { auto: true },
  ],
  app: [
    async ({}, use) => {
      killStaleInstances();
      const dataDir = seedHermeticDataDir();
      const child = spawnApp(dataDir);
      let browser: Browser | undefined;
      try {
        await waitForCdp();
        browser = await chromium.connectOverCDP(cdpBaseUrl);
        const context = browser.contexts()[0] ?? (await browser.newContext());
        const page = await findAppPage(context);
        await page.waitForLoadState("domcontentloaded");
        const noise = attachNoiseCollector(page);
        await use({ page, noise });
      } finally {
        if (browser) await browser.close().catch(() => undefined);
        child.kill();
        killStaleInstances();
        rmSync(dataDir, { recursive: true, force: true });
      }
    },
    { scope: "worker", timeout: appReadyTimeoutMs + 10_000 },
  ],
});

export { expect };
export type { Page };
