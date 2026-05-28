import type { Release } from "./api";

/// Merge the release lists of every DLL family that maps to one catalog feature
/// (e.g. FSR Upscaling = DX12 + Vulkan), de-duped by version+hash and sorted
/// newest-first by packed version.
export function mergeFamilyReleases(lists: Release[][]): Release[] {
  const seen = new Set<string>();
  const merged: Release[] = [];
  for (const list of lists) {
    for (const r of list) {
      const key = `${r.version}|${r.sha256}`;
      if (!seen.has(key)) {
        seen.add(key);
        merged.push(r);
      }
    }
  }
  return merged.sort((a, b) => Number(b.version_packed ?? 0) - Number(a.version_packed ?? 0));
}
