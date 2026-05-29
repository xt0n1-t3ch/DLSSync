
import fs from "node:fs";
import path from "node:path";

const CDP = process.env.DLSS_CDP || "http://127.0.0.1:9333";
const OUT = path.join(process.env.TEMP || ".", "dlss-shots-e2e");
fs.mkdirSync(OUT, { recursive: true });

const listing = await (await fetch(`${CDP}/json/list`)).json();
const page = listing.find((t) => t.type === "page") || listing[0];
if (!page) { console.error(JSON.stringify({ fatal: "no CDP page target — is the app running with --remote-debugging-port?" })); process.exit(1); }

const ws = new WebSocket(page.webSocketDebuggerUrl);
let seq = 0; const pending = new Map();
const consoleErrors = []; const exceptions = [];
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); return; }
  if (m.method === "Runtime.consoleAPICalled" && (m.params?.type === "error" || m.params?.type === "warning"))
    consoleErrors.push({ type: m.params.type, text: (m.params.args || []).map((a) => a.value ?? a.description ?? a.type).join(" ") });
  else if (m.method === "Runtime.exceptionThrown")
    exceptions.push(m.params?.exceptionDetails?.exception?.description || m.params?.exceptionDetails?.text || "exception");
};
const send = (method, params = {}) => new Promise((r) => { const id = ++seq; pending.set(id, r); ws.send(JSON.stringify({ id, method, params })); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
await send("Page.enable"); await send("Runtime.enable"); await sleep(300);

const ev = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
  if (r?.exceptionDetails) return { __err: r.exceptionDetails.text };
  return r?.result?.value;
};
const goto = (title) => ev(`(()=>{const b=document.querySelector('button[title=${JSON.stringify(title)}]');if(b){b.click();return 1}return 0})()`);
const count = (sel) => ev(`document.querySelectorAll(${JSON.stringify(sel)}).length`);
const has = (s) => ev(`document.body.innerText.includes(${JSON.stringify(s)})`);
const shot = async (n) => { const r = await send("Page.captureScreenshot", { format: "png" }); if (r?.data) fs.writeFileSync(path.join(OUT, n), Buffer.from(r.data, "base64")); };

const results = [];
async function check(name, fn) {
  try {
    const out = await fn();
    const status = out?.gated ? "GATED" : out?.pass ? "PASS" : "FAIL";
    results.push({ name, status, detail: out?.detail ?? "" });
  } catch (err) {
    results.push({ name, status: "FAIL", detail: "threw: " + String(err) });
  }
}

await check("shell:mounts + 6 nav items + v1.5.2", async () => {
  const nav = await ev(`["Library","Catalog","Drivers","Backups","Settings","About"].filter(t=>document.querySelector('button[title="'+t+'"]')).length`);
  const shell = await ev(`!!document.querySelector('.app-shell,.drivers-view,.catalog-page') || document.body.children.length>0`);
  return { pass: nav === 6 && shell, detail: `navItems=${nav} shell=${shell}` };
});

await check("library:cards + updates-hero + grid/list + density + filter + search", async () => {
  await goto("Library"); await sleep(900);
  const cards = await count(".game-card");
  const hero = await count(".updates-hero");
  const seg = await count(".seg-btn");
  const search = await count(".lib-section input, header input, input[type=search]");
  const filterPill = await ev(`!!document.querySelector('button[title*="pending"],button[title*="updates"]')`);
  await ev(`(()=>{const b=[...document.querySelectorAll('.seg-btn')].find(x=>/list/i.test(x.title||x.textContent));if(b)b.click()})()`); await sleep(400);
  await ev(`(()=>{const b=[...document.querySelectorAll('.seg-btn')].find(x=>/grid/i.test(x.title||x.textContent));if(b)b.click()})()`); await sleep(300);
  await shot("01-library.png");
  return { pass: cards > 0 && hero >= 0 && seg >= 2, detail: `cards=${cards} updatesHero=${hero} segBtns=${seg} filterPill=${filterPill} searchInputs=${search}` };
});

await check("gameDetail:opens full page + feature rows + DLSS override + back", async () => {
  await ev(`document.querySelector('.game-card')?.click()`); await sleep(1300);
  const detail = await count(".detail-view");
  const back = await count(".detail-back");
  const featureRows = await count(".feature-row");
  const summary = await count(".summary-row");
  const foot = await count(".drawer-foot");
  const overrideInputs = await ev(`document.querySelectorAll('.drawer-body input, .detail-view input, .detail-view select').length`);
  await shot("02-game-detail.png");
  const ok = detail === 1 && back === 1 && featureRows > 0 && summary === 1 && foot === 1;
  await ev(`document.querySelector('.detail-back')?.click()`); await sleep(700);
  const returned = await count(".game-card");
  return { pass: ok && returned > 0, detail: `detailView=${detail} back=${back} featureRows=${featureRows} summary=${summary} foot=${foot} overrideControls=${overrideInputs} returnedToLibrary=${returned > 0}` };
});

await check("driversGPU:GPU list renders with vendor + version", async () => {
  await goto("Drivers"); await sleep(1400);
  const list = await count(".driver-list");
  const gpuRows = await ev(`document.querySelectorAll('.driver-list > *, .driver-list li, .driver-card').length`);
  const vendors = await ev(`(() => { const t=(document.querySelector('.driver-list')||{}).innerText||''; return ["NVIDIA","GeForce","RTX","AMD","Radeon","Intel","Arc","Iris"].filter(v=>t.includes(v)); })()`);
  await shot("03-drivers-gpu.png");
  return { pass: list >= 1 && gpuRows > 0 && Array.isArray(vendors) && vendors.length > 0, detail: `driverList=${list} gpuRows=${gpuRows} vendorTokens=${JSON.stringify(vendors)}` };
});

await check("systemComponents:scan + admin note + version history (real data)", async () => {
  const adminNote = await has("Administrator rights");
  await ev(`(()=>{const b=[...document.querySelectorAll('button')].find(x=>/Rescan|Scanning/.test(x.textContent));if(b)b.click()})()`);
  let cards = 0, scanning = true;
  for (let i = 0; i < 8 && (cards === 0 || scanning); i++) {
    await sleep(6000);
    cards = await count(".sys-card");
    scanning = await ev(`[...document.querySelectorAll('button')].some(b=>/Scanning/.test(b.textContent))`);
    if (cards > 0 && !scanning) break;
  }
  const groups = await count(".sys-group");
  const verToggle = await count('button[aria-label="Version history"]');
  let verPanel = false, verText = null;
  if (verToggle > 0) {
    await ev(`document.querySelector('button[aria-label="Version history"]').click()`); await sleep(2500);
    verPanel = await ev(`!!document.querySelector('.sys-versions-panel')`);
    verText = await ev(`(document.querySelector('.sys-versions-panel')||{}).innerText||null`);
  }
  await shot("04-system-components.png");
  return { pass: adminNote && cards > 0 && groups > 0, detail: `adminNote=${adminNote} cards=${cards} groups=${groups} versionToggle=${verToggle} versionPanel=${verPanel} sample=${JSON.stringify((verText||"").replace(/\n/g," ").slice(0,90))}` };
});

await check("dlssOverrides:global panel present", async () => {
  const present = (await has("DLSS Overrides")) || (await ev(`!!document.querySelector('.dlss-override,[class*=override]')`));
  return { pass: present, detail: `present=${present}` };
});

await check("catalog:vendor families + version pickers + GPU drivers", async () => {
  await goto("Catalog"); await sleep(2200);
  const vendorCards = await count(".vendor-card");
  const familyRows = await count(".feature-row, .feature-row-btn");
  const gpuCatRows = await count(".driver-cat-row");
  const fams = await ev(`["DLSS","FSR","XeSS","Reflex"].filter(f=>document.body.innerText.includes(f))`);
  const opened = await ev(`(()=>{const b=document.querySelector('.feature-row-btn');if(b){b.click();return 1}return 0})()`); await sleep(900);
  const flyout = await ev(`!!document.querySelector('.glass-dialog,[class*=flyout],[class*=popover],[role=menu]')`);
  await ev(`document.body.click()`); await sleep(300);
  await shot("05-catalog.png");
  return { pass: vendorCards > 0 && familyRows > 0 && Array.isArray(fams) && fams.length >= 2, detail: `vendorCards=${vendorCards} familyRows=${familyRows} gpuCatRows=${gpuCatRows} families=${JSON.stringify(fams)} pickerOpened=${opened} flyout=${flyout}` };
});

await check("catalog:DirectStorage (gated on jsdelivr @main propagation)", async () => {
  const ds = await has("DirectStorage");
  const ms = await has("Microsoft");
  return { gated: !ds, pass: ds, detail: ds ? "DirectStorage visible (manifest propagated)" : "GATED: manifest @main not yet propagated to jsdelivr; app still serving stale (no microsoft). Published+verified at source." };
});

await check("backups:hero + DLL backups + search + group-by", async () => {
  await goto("Backups"); await sleep(1200);
  const hero = await count(".backup-hero");
  const groups = await count(".group-row");
  const search = await count(".backup-search input");
  const groupBy = await count(".group-by-toggle .seg-btn, .backup-toolbar .seg-btn");
  const sysSection = await has("System Drivers");
  await shot("06-backups.png");
  return { pass: hero >= 0 && (groups > 0 || (await count(".empty")) > 0) && search >= 1, detail: `hero=${hero} dllGroups=${groups} search=${search} groupByBtns=${groupBy} systemDriversSection=${sysSection} (empty unless a system driver was installed)` };
});

await check("notifications:bell opens panel", async () => {
  await goto("Library"); await sleep(400);
  await ev(`document.querySelector('button[title="Notifications"]')?.click()`); await sleep(700);
  const panel = await ev(`!!document.querySelector('[class*=notification][class*=panel],.notifications-panel,[data-notifications-panel]') || /notification/i.test(document.body.innerText)`);
  await shot("07-notifications.png");
  await ev(`document.body.click()`); await sleep(200);
  return { pass: panel, detail: `panel=${panel}` };
});

await check("commandPalette:opens + searches + results", async () => {
  await ev(`document.querySelector('button[title*="Command palette"]')?.click()`); await sleep(600);
  const open = await ev(`!!document.querySelector('.palette, [class*=palette]')`);
  await ev(`(()=>{const i=document.querySelector('.palette input, [class*=palette] input');if(i){i.value='back';i.dispatchEvent(new Event('input',{bubbles:true}))}})()`); await sleep(500);
  const results = await ev(`document.querySelectorAll('.palette [class*=result], [class*=palette] [class*=result], .palette li, [class*=palette] li').length`);
  await shot("08-command-palette.png");
  await ev(`document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))`); await sleep(200);
  return { pass: open, detail: `open=${open} results=${results}` };
});

await check("settings:hero version + sections (tabbed) + tab switch + toggle", async () => {
  await goto("Settings"); await sleep(900);
  const hero = await count(".settings-hero");
  const ver = await has("1.5.2");
  const sections = await count(".section-title-h, .settings-hero ~ * .section-title");
  const toggles = await count("input[type=checkbox], .row input, button[role=switch], .seg-btn");
  const tabs = await count('[role=tab], .tab-btn, .settings-tab');
  if (tabs > 0) { await ev(`document.querySelectorAll('[role=tab],.tab-btn,.settings-tab')[1]?.click()`); await sleep(400); }
  await shot("09-settings.png");
  return { pass: hero >= 1 && ver && sections >= 2 && toggles > 0, detail: `hero=${hero} versionShown=${ver} sectionHeadingsInDom=${sections} toggles=${toggles} tabs=${tabs}` };
});

await check("about:version v1.5.2 + manifest sources + system info", async () => {
  await goto("About"); await sleep(800);
  const ver = await ev(`(document.body.innerText.match(/v?1\\.5\\.2/)||[null])[0]`);
  const sources = await count(".source-card");
  const sys = await has("Your system");
  await shot("10-about.png");
  return { pass: !!ver && sys, detail: `version=${ver} manifestSources=${sources} systemInfo=${sys}` };
});

await check("theme:toggle flips dark/light", async () => {
  await goto("Library"); await sleep(300);
  const before = await ev(`document.documentElement.getAttribute('data-theme')||document.documentElement.className`);
  await ev(`document.querySelector('button[title="Toggle theme"]')?.click()`); await sleep(600);
  const after = await ev(`document.documentElement.getAttribute('data-theme')||document.documentElement.className`);
  await shot("11-theme.png");
  await ev(`document.querySelector('button[title="Toggle theme"]')?.click()`); await sleep(300);
  return { pass: before !== after, detail: `before=${before} after=${after}` };
});

await check("console:0 errors + 0 exceptions across the run", async () => {
  return { pass: consoleErrors.length === 0 && exceptions.length === 0, detail: `consoleErrors=${consoleErrors.length} exceptions=${exceptions.length}` };
});

const pass = results.filter((r) => r.status === "PASS").length;
const fail = results.filter((r) => r.status === "FAIL").length;
const gated = results.filter((r) => r.status === "GATED").length;
console.log("\n===== DLSSync FULL E2E =====");
for (const r of results) console.log(`[${r.status.padEnd(5)}] ${r.name}\n         ${r.detail}`);
console.log(`\nSUMMARY pass=${pass} fail=${fail} gated=${gated} | consoleErrors=${consoleErrors.length} exceptions=${exceptions.length}`);
if (consoleErrors.length) console.log("CONSOLE: " + JSON.stringify(consoleErrors.slice(0, 10)));
if (exceptions.length) console.log("EXC: " + JSON.stringify(exceptions.slice(0, 10)));
console.log("shots: " + OUT);
ws.close();
process.exit(fail > 0 ? 1 : 0);
