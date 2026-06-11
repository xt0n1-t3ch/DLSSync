import { describe, it, expect } from "vitest";
import { detectRevertedSwaps } from "@/lib/revertDetect";
import type { BackupEntry, DetectedGame, DllRecord } from "@/lib/api";

const GAME: DetectedGame = {
  id: "steam-1091500",
  name: "Cyberpunk 2077",
  launcher: "steam",
  install_dir: "C:\\Games\\Cyberpunk 2077",
  app_id: "1091500",
  image_url: null,
  size_bytes: null,
};

function record(version: string | null, path = "C:\\Games\\Cyberpunk 2077\\nvngx_dlss.dll"): DllRecord {
  return {
    family: "dlss_sr",
    path,
    current_version: version,
    file_description: null,
    sha256: null,
  };
}

function backup(overrides: Partial<BackupEntry> = {}): BackupEntry {
  return {
    id: "b1",
    game_id: GAME.id,
    dll_family: "dlss_sr",
    dll_filename: "nvngx_dlss.dll",
    original_path: "C:\\Games\\Cyberpunk 2077\\nvngx_dlss.dll",
    backup_path: "D:\\DLSSync\\Backups\\cp77\\nvngx_dlss.dll",
    previous_version: "310.5.0.0",
    previous_sha256: null,
    created_at: "2026-06-01T00:00:00Z",
    restored_at: null,
    size_bytes: null,
    backup_type: "game_dll",
    device_class: null,
    hardware_id: null,
    ...overrides,
  } as BackupEntry;
}

describe("detectRevertedSwaps", () => {
  it("flags a swap whose on-disk version regressed to the backed-up previous version", () => {
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [record("310.5.0.0")] }, [backup()]);
    expect(reverted).toHaveLength(1);
    expect(reverted[0].game.id).toBe(GAME.id);
    expect(reverted[0].backup.id).toBe("b1");
  });

  it("ignores a healthy swap still carrying the newer version", () => {
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [record("310.6.0.0")] }, [backup()]);
    expect(reverted).toEqual([]);
  });

  it("ignores manually-restored backups and driver packages", () => {
    const restored = backup({ restored_at: "2026-06-05T00:00:00Z" });
    const driver = backup({ id: "b2", backup_type: "driver_package" });
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [record("310.5.0.0")] }, [restored, driver]);
    expect(reverted).toEqual([]);
  });

  it("uses only the NEWEST backup per path so a re-apply clears the flag", () => {
    const stale = backup({ id: "old", previous_version: "310.5.0.0", created_at: "2026-06-01T00:00:00Z" });
    const fresh = backup({ id: "new", previous_version: "310.6.0.0", created_at: "2026-06-08T00:00:00Z" });
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [record("310.5.0.0")] }, [stale, fresh]);
    expect(reverted).toEqual([]);
  });

  it("matches paths case-insensitively and across slash styles", () => {
    const rec = record("310.5.0.0", "c:/games/cyberpunk 2077/NVNGX_DLSS.DLL");
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [rec] }, [backup()]);
    expect(reverted).toHaveLength(1);
  });

  it("ignores records with no readable version", () => {
    const reverted = detectRevertedSwaps([GAME], { [GAME.id]: [record(null)] }, [backup()]);
    expect(reverted).toEqual([]);
  });
});
