const DISTRIBUTION = import.meta.env.VITE_DLSSYNC_DISTRIBUTION ?? "standard";

export const isNexusBuild = DISTRIBUTION === "nexus";
export const appUpdaterEnabled = !isNexusBuild;
export const distributionLabel = isNexusBuild ? "Nexus Mods" : "Standard";

// In the Nexus build, hide every link that points at the DLSSync GitHub
// repository (releases, issues, sponsors, the author profile, the star CTA).
// Nexus Mods does not allow linking to source/auto-updating builds from a
// hosted mod, so the Nexus package surfaces none of them. Upstream vendor SDK
// links (NVIDIA/Intel/AMD/Microsoft) and non-GitHub links (Ko-fi, Discord,
// website, the Nexus page itself) are unaffected.
export const showSourceLinks = !isNexusBuild;
