import { defineConfig, devices } from "@playwright/test";
import { reportDir, resultsDir } from "./config";

export default defineConfig({
  testDir: ".",
  globalSetup: "./global-setup.ts",
  outputDir: resultsDir,
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 2 : 0,
  reporter: [["list"], ["html", { outputFolder: reportDir, open: "never" }]],
  use: {
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
