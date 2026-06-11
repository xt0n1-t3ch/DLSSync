import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const script = readFileSync(resolve(__dirname, "../../scripts/kill-dlssync.mjs"), "utf8");

describe("kill-dlssync safety", () => {
  it("does not use a broad image-name taskkill", () => {
    expect(script).not.toMatch(/taskkill\s+\/IM\s+dlssync\.exe/i);
  });

  it("kills only repo target binaries discovered by executable path", () => {
    expect(script).toContain("allowedExecutablePaths");
    expect(script).toContain("target\", \"debug\", \"dlssync.exe");
    expect(script).toContain("target\", \"release\", \"dlssync.exe");
    expect(script).toContain("ExecutablePath");
    expect(script).toContain("process.kill(pid)");
  });
});
