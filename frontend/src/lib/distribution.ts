const DISTRIBUTION = import.meta.env.VITE_DLSSYNC_DISTRIBUTION ?? "standard";

export const isNexusBuild = DISTRIBUTION === "nexus";
export const appUpdaterEnabled = !isNexusBuild;
export const distributionLabel = isNexusBuild ? "Nexus Mods" : "Standard";
