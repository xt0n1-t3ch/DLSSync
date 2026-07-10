<script lang="ts">
  import { pendingUpdatePlan, completeUpdatePlan } from "../features/update-plan/model";
  import { getCatalogStatus, listReleases, type CatalogRuntimeStatus, type Release } from "../lib/api";
  import { familyCatalogKey, familyVendor } from "../lib/labels";
  import { t, locale } from "../lib/i18n/index";

  type Evidence = { source: string; signer: string | null; sha256: string; signed: boolean };
  let selected = $state<Record<string, boolean>>({});
  let evidence = $state<Record<string, Evidence>>({});
  let status = $state<CatalogRuntimeStatus | null>(null);
  let loading = $state(false);
  let loadedKey = $state("");

  const targetKey = (path: string, version: string): string => `${path}::${version}`;

  $effect(() => {
    const pending = $pendingUpdatePlan;
    if (!pending) {
      loadedKey = "";
      return;
    }
    const key = pending.targets.map((target) => targetKey(target.record.path, target.target_version)).join("|");
    if (key === loadedKey) return;
    loadedKey = key;
    selected = Object.fromEntries(pending.targets.map((target) => [targetKey(target.record.path, target.target_version), true]));
    void loadEvidence(pending.targets);
  });

  async function loadEvidence(targets: NonNullable<typeof $pendingUpdatePlan>["targets"]): Promise<void> {
    loading = true;
    try {
      status = await getCatalogStatus();
      const rows = await Promise.all(targets.map(async (target) => {
        const vendor = familyVendor(target.record.family);
        const family = familyCatalogKey(target.record.family);
        const releases = await listReleases(vendor, family);
        const filename = target.record.path.split(/[\\/]/).pop()?.toLowerCase();
        const release = releases.find((candidate: Release) =>
          candidate.version === target.target_version && candidate.filename.toLowerCase() === filename,
        ) ?? releases.find((candidate: Release) => candidate.version === target.target_version);
        return [targetKey(target.record.path, target.target_version), {
          source: release?.source ?? "—",
          signer: release?.signature_subject ?? null,
          sha256: release?.sha256 ?? "—",
          signed: release?.signed ?? false,
        }] as const;
      }));
      evidence = Object.fromEntries(rows);
    } finally {
      loading = false;
    }
  }

  function selectedTargets(): NonNullable<typeof $pendingUpdatePlan>["targets"] {
    return ($pendingUpdatePlan?.targets ?? []).filter((target) => selected[targetKey(target.record.path, target.target_version)]);
  }

  function apply(): void {
    if (!status) return;
    const targets = selectedTargets();
    if (targets.length === 0) return;
    completeUpdatePlan({ targets, catalogGeneratedAt: status.provenance.generated_at });
  }

  async function exportPlan(): Promise<void> {
    if (!status) return;
    const items = selectedTargets().map((target) => ({
      game_id: target.game_id,
      game: target.game_label,
      file: target.record.path,
      current_version: target.record.current_version,
      target_version: target.target_version,
      backup: true,
      trust: evidence[targetKey(target.record.path, target.target_version)] ?? null,
    }));
    await navigator.clipboard.writeText(JSON.stringify({ catalog_generated_at: status.provenance.generated_at, items }, null, 2));
  }
</script>

{#if $pendingUpdatePlan}
  <div class="plan-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) completeUpdatePlan(null); }}>
    <div class="plan-modal" role="dialog" aria-modal="true" aria-labelledby="update-plan-title">
      <header>
        <div><span class="eyebrow">{$t("component.updatePlan.eyebrow")}</span><h2 id="update-plan-title">{$t("component.updatePlan.title")}</h2><p>{$t("component.updatePlan.subtitle")}</p></div>
        <button class="close" aria-label={$t("common.close")} onclick={() => completeUpdatePlan(null)}>×</button>
      </header>
      <div class="plan-proof">
        <span class="proof-item" class:is-verified={status?.provenance.signature_verified} data-testid="plan-proof-signature">
          <svg class="proof-glyph" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="m9 12 2 2 4-4"/></svg>
          {status?.provenance.signature_verified ? "✓" : "—"} Ed25519
        </span>
        <span class="proof-item">
          <svg class="proof-glyph" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><polyline points="12 7 12 12 15 14"/></svg>
          {status ? new Date(status.provenance.generated_at).toLocaleString($locale) : $t("component.updatePlan.loading")}
        </span>
        <span class="proof-item">
          <svg class="proof-glyph" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
          {$pendingUpdatePlan.targets.length} {$t("component.updatePlan.files")}
        </span>
      </div>
      <div class="plan-list">
        {#each $pendingUpdatePlan.targets as target (targetKey(target.record.path, target.target_version))}
          {@const key = targetKey(target.record.path, target.target_version)}
          {@const proof = evidence[key]}
          <label class="plan-row" class:is-selected={selected[key]}>
            <input type="checkbox" bind:checked={selected[key]}>
            <div class="file-main">
              <strong class="file-name">{target.game_label}</strong>
              <code class="file-path" title={target.record.path}>{target.record.path}</code>
              <div class="version-delta" aria-label="{target.record.current_version ?? '?'} → {target.target_version}">
                <span class="v-from mono">{target.record.current_version ?? "?"}</span>
                <svg class="v-arrow" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                <span class="v-to mono">{target.target_version}</span>
              </div>
            </div>
            <div class="file-trust">
              <span class="trust-line signer" class:is-signed={proof?.signed}>
                <svg class="trust-glyph" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>{#if proof?.signed}<path d="m9 12 2 2 4-4"/>{/if}</svg>
                <span class="trust-text truncate">{proof?.signer ?? $t("component.updatePlan.signerPending")}</span>
              </span>
              <span class="trust-line">
                <svg class="trust-glyph" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="4" y1="9" x2="20" y2="9"/><line x1="4" y1="15" x2="20" y2="15"/><line x1="10" y1="3" x2="8" y2="21"/><line x1="16" y1="3" x2="14" y2="21"/></svg>
                <code class="trust-text mono">{proof?.sha256.slice(0, 16) ?? "—"}…</code>
              </span>
              <span class="trust-line">
                <svg class="trust-glyph" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></svg>
                <span class="trust-text truncate">{proof?.source ?? "—"}</span>
              </span>
            </div>
          </label>
        {/each}
      </div>
      <footer><button class="btn btn-ghost" onclick={exportPlan} disabled={!status}>{$t("component.updatePlan.export")}</button><div class="actions"><button class="btn btn-ghost" onclick={() => completeUpdatePlan(null)}>{$t("common.cancel")}</button><button class="btn btn-primary" onclick={apply} disabled={loading || !status || selectedTargets().length === 0}>{$t("component.updatePlan.apply")}</button></div></footer>
    </div>
  </div>
{/if}

<style>
  .plan-backdrop{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;padding:24px;background:rgba(3,5,9,.72);backdrop-filter:blur(14px)}
  .plan-modal{width:min(960px,96vw);max-height:88vh;display:flex;flex-direction:column;overflow:hidden;border:1px solid var(--border);border-radius:var(--radius-2xl);background:var(--bg-elevated);box-shadow:var(--shadow-lg)}
  header{display:flex;justify-content:space-between;gap:24px;padding:22px 26px 14px}h2{margin:3px 0;font-size:var(--fs-2xl);letter-spacing:var(--letter-tighter)}header p{margin:0;color:var(--text-muted);font-size:var(--fs-sm)}.eyebrow{color:var(--accent);font-size:var(--fs-xs);font-weight:800;text-transform:uppercase;letter-spacing:.12em}.close{border:0;background:transparent;color:var(--text-muted);font-size:28px;line-height:1;cursor:pointer;border-radius:var(--radius-md);width:32px;height:32px}.close:hover{background:var(--bg-input);color:var(--text-primary)}

  .plan-proof{display:flex;gap:8px;flex-wrap:wrap;padding:0 26px 18px}
  .plan-proof .proof-item{display:inline-flex;align-items:center;gap:6px;padding:6px 12px;border-radius:var(--radius-full);background:var(--bg-input);border:1px solid var(--border);color:var(--text-secondary);font-size:var(--fs-xs);font-weight:600;font-variant-numeric:tabular-nums}
  .plan-proof .proof-glyph{color:var(--text-muted);flex-shrink:0}
  .plan-proof .proof-item.is-verified{background:var(--success-dim);border-color:color-mix(in oklab,var(--success) 32%,transparent);color:var(--success)}
  .plan-proof .proof-item.is-verified .proof-glyph{color:var(--success)}

  .plan-list{overflow:auto;padding:0 22px 14px;display:flex;flex-direction:column;gap:10px}
  .plan-row{display:grid;grid-template-columns:auto minmax(0,1.4fr) minmax(220px,0.9fr);gap:16px;align-items:center;padding:14px 16px;border:1px solid var(--border);border-radius:var(--radius-lg);background:var(--bg-card);transition:border-color var(--dur-fast) var(--ease),background var(--dur-fast) var(--ease)}
  .plan-row:hover{border-color:var(--border-hover)}
  .plan-row.is-selected{border-color:color-mix(in oklab,var(--accent) 42%,var(--border));background:color-mix(in oklab,var(--accent) 5%,var(--bg-card))}
  .plan-row input{width:18px;height:18px;flex-shrink:0;accent-color:var(--accent)}
  .file-main{min-width:0;display:flex;flex-direction:column;gap:7px}
  .file-name{font-size:var(--fs-base);font-weight:650;color:var(--text-primary);letter-spacing:var(--letter-tight);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .file-path{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--text-muted);font-family:var(--font-mono);font-size:var(--fs-2xs)}
  .version-delta{display:inline-flex;align-items:center;gap:9px;align-self:flex-start;padding:4px 12px;border-radius:var(--radius-full);background:var(--bg-input);border:1px solid var(--border);font-variant-numeric:tabular-nums}
  .version-delta .v-from{color:var(--text-muted);font-size:var(--fs-sm);text-decoration:line-through;text-decoration-color:color-mix(in oklab,var(--text-muted) 55%,transparent)}
  .version-delta .v-arrow{color:var(--text-placeholder);flex-shrink:0}
  .version-delta .v-to{color:var(--success);font-weight:700;font-size:var(--fs-sm)}

  .file-trust{min-width:0;display:flex;flex-direction:column;gap:6px}
  .trust-line{display:inline-flex;align-items:center;gap:7px;min-width:0;color:var(--text-muted);font-size:var(--fs-xs)}
  .trust-glyph{color:var(--text-placeholder);flex-shrink:0}
  .trust-text{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-width:0}
  .trust-line.signer .trust-text{color:var(--text-secondary);font-weight:600}
  .trust-line.signer.is-signed{color:var(--success)}
  .trust-line.signer.is-signed .trust-glyph{color:var(--success)}
  .trust-line.signer.is-signed .trust-text{color:var(--success)}
  .trust-line code.trust-text{font-size:var(--fs-2xs)}

  footer{display:flex;justify-content:space-between;gap:12px;padding:16px 24px;border-top:1px solid var(--border);background:var(--bg-elevated)}.actions{display:flex;gap:10px}
  @media(max-width:760px){.plan-row{grid-template-columns:auto 1fr;row-gap:12px}.file-trust{grid-column:2}.version-delta{align-self:flex-start}footer{align-items:stretch;flex-direction:column}.actions{display:grid;grid-template-columns:1fr 1fr}}
</style>
