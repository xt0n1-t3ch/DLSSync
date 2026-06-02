# Translating DLSSync

Want DLSSync in your language? You do not need to be a programmer, and you do not
need to touch a single line of code. Every word the app shows on screen lives in a
plain JSON file. You copy the English file, replace the English text with your
language, run two checks, and open a pull request. That's the whole job.

## Overview / TL;DR

- All UI text lives in `frontend/src/lib/i18n/locales/<locale>.json` — one file per
  language. Today there are two: `en.json` (English) and `es.json` (Spanish).
- **`en.json` is the source of truth.** Every other catalog must have exactly the
  same set of keys, in the same nesting. If English has a key, your language needs it
  too; if English doesn't, you can't invent one.
- To translate, you copy the English **values** and rewrite them in your language. You
  never rename a key, never delete a key, and never touch any `.ts` or `.svelte` file
  to do a translation.
- Two commands prove your work is correct before you submit. Both run offline.

That's it. The rest of this page is detail you can skim and come back to.

## Where each kind of string lives

| What | Lives in | You edit it? |
|:---|:---|:---|
| In-app UI text (buttons, labels, tooltips, toasts, notifications) | `frontend/src/lib/i18n/locales/<locale>.json` | Yes — this is the file you translate |
| What a string *means*, where it's shown, and which words to leave alone | `frontend/src/lib/i18n/locales/_meta.json` | No — read it for guidance, keyed off the English text |
| Brand and product names (see below) | Stay English in **every** locale, including `en.json` | No — leave them exactly as written |

`_meta.json` is a sidecar, not a catalog. Each entry is keyed by the English key
path and tells you the English source text (`en`), the intent (`desc`), and where the
string appears (`where`). For example:

```json
"errorClass.signature.hint": {
  "en": "Vendor ships this DLL unsigned, or the embedded subject is not on the allowlist. Toggle 'Allow unsigned DLLs' in Settings → Advanced and retry — SHA-256 is still enforced.",
  "desc": "Recovery hint for a signature apply failure. Keep DLL/SHA-256 and the 'Allow unsigned DLLs' setting name + Settings → Advanced path; translate the rest.",
  "where": "ApplyProgressModal error detail."
}
```

When you hit a string whose meaning isn't obvious, look it up in `_meta.json` first.

### Brand and product names — keep these in English

These are proper nouns in the GPU and PC-gaming domain. They are not English words
that happen to have a Spanish equivalent — they are the actual names of products and
companies. Translating them makes the app *wrong*, not localized. Leave them verbatim
in `es.json`, `en.json`, and any future locale:

> **NVIDIA, AMD, Intel, Microsoft, DLSS, FSR, XeSS, Reflex, Streamline, DirectStorage,
> DLSSync** — plus launcher names (**Steam, Epic, GOG, Ubisoft, EA, Xbox, Battle.net**),
> the preset letters (Preset K, Preset L…), the model families (Transformer, CNN), and
> tokens like RTX, NVAPI, FP8, DLAA, V-Sync, SHA-256, WHQL.

A real example from `es.json` — only the descriptive words changed, the brand stayed:

```json
"feature.dlss_sr": {
  "blurb": "Imagen más nítida a más FPS — escalado por IA de NVIDIA",
  "title": "DLSS Super Resolución"
}
```

`NVIDIA` and `DLSS` survive untouched; `Sharper image at higher FPS — NVIDIA AI
upscaling` becomes Spanish around them. The `desc` line in `_meta.json` calls out
exactly which tokens to keep for each string.

## Anatomy of a catalog file

A catalog is one big nested JSON object. Keys group by area, then component, then
purpose — `area.component.purpose`. Reading top to bottom you can usually tell where a
string shows up:

```json
{
  "view": {
    "library": {
      "title": "Library",
      "toast": {
        "gameHidden": "{name} hidden"
      }
    }
  }
}
```

That nesting is the key path `view.library.toast.gameHidden`. The app looks strings up
by that dotted path, so the **shape** of the JSON is load-bearing — two locales must
nest identically.

Rules that keep the catalog sane:

- **One key per UI instance. Never reuse a key in two places.** It's tempting to share
  `common.close` everywhere a "Close" button exists, and `common.*` does hold a few
  genuinely-shared atoms (Cancel, Close, Apply). But for anything with context — a
  heading, a tooltip, a toast — give it its own key under its own component. The reason
  is grammar: English "Open" is one word, but a Spanish translation might need "Abrir"
  in one spot and "Abierto" in another. Shared keys make that impossible.
- **Don't rename or reorder keys** to translate. The key is the address. Change the
  value, leave the address alone.
- **Casing matters in values.** `SYNC · VERIFY · APPLY` is intentionally uppercase;
  `Up to date` is sentence case. Match the style of the English value unless your
  language has its own convention (Spanish, for instance, doesn't capitalize "lista"
  mid-sentence).

## Placeholders and plurals

### Placeholders: `{name}`, `{count}`, `{error}`

A value can contain `{single-brace}` slots. At runtime the app swaps in a real value:

```json
"view.library.subtitle": "{detected} games detected — {shown} shown"
```

becomes `42 games detected — 12 shown`. **Copy every placeholder into your translation
exactly — same spelling, same braces.** You can move them around to fit your grammar,
you just can't drop one or rename it. The Spanish version reorders the sentence but
keeps both slots:

```json
"view.library.subtitle": "{detected} juegos detectados — {shown} mostrados"
```

If you write `{detected}` as `{detectado}`, the slot breaks and the app prints the
literal text `{detectado}` to the user.

### Plurals: the `_one` / `_other` suffix

English has two number forms (1 item, 2 items). The catalog encodes that with a
**suffix on the key** plus a `{count}` placeholder. The engine reads `count`, asks
`Intl.PluralRules` which category your language wants, and picks the matching suffix.

Here's the real pair from `en.json` and `es.json` for `common.updatesReady`:

```json
// en.json
"common": {
  "updatesReady_one": "{count} update ready",
  "updatesReady_other": "{count} updates ready"
}
```

```json
// es.json
"common": {
  "updatesReady_one": "{count} actualización lista",
  "updatesReady_other": "{count} actualizaciones listas"
}
```

The code only ever asks for `common.updatesReady` with a `count`; the engine appends
`_one` or `_other` for you. `_meta.json` flags these — its entry is keyed at the base
`common.updatesReady` and the `desc` says "Plural… Provide `_one`… and `_other`…".

**Other languages may need more forms.** `Intl.PluralRules` defines six categories:
`_zero`, `_one`, `_two`, `_few`, `_many`, `_other`. English and Spanish only use
`_one` and `_other`. If you add a language with richer plural rules — Polish, Russian,
Arabic — provide every form your language's rules select, and `_other` as the fallback.
You can check what your language needs:

```js
new Intl.PluralRules("pl").select(2)  // -> "few"  (Polish needs _few)
```

## How to translate (the contributor loop)

For an existing language, say Spanish:

1. Open `frontend/src/lib/i18n/locales/es.json` side by side with `en.json`.
2. Find a value still in English (or one you want to improve).
3. Look up that key in `frontend/src/lib/i18n/locales/_meta.json` — read `desc` for
   the intent and which tokens to keep, and `where` for the on-screen location.
4. Replace the English value with your translation. Keep the key identical. Keep every
   `{placeholder}` identical. Keep the brand names from the list above in English.
5. Never leave a value empty (`""`) and never delete the key. If you genuinely can't
   translate something yet, leave the English value in place — an untranslated string
   ships fine; an empty or missing one fails the build.
6. Save. Run the two checks below.

## How to verify (locally)

From the repo root, run both. They are fast and offline.

```bash
pnpm --filter dlssync-frontend check
```

This is `svelte-check` — the TypeScript compile pass. The file
`frontend/src/lib/i18n/_parity.ts` assigns each non-English catalog into the
`Messages` type (which is derived from `en.json`):

```ts
const _esParity: Messages = es;
```

If `es.json` is **missing a key** that `en.json` has, the types don't match and this
command fails at compile time with a `TS2741` error. So `check` is your "did I delete
or forget a key" guard.

```bash
pnpm --filter dlssync-frontend test
```

This runs Vitest, including `tests/unit/i18nParity.test.ts`. That test flattens every
catalog to dotted keys and asserts three things against `en.json`:

- **No missing keys** — same coverage the compile check gives you, but with a readable
  list of exactly which keys are absent.
- **No extra keys** — a key in `es.json` that isn't in `en.json` (a typo'd key name, a
  leftover) fails here. The compiler can't catch this one; the test can.
- **No empty values** — every value, in every catalog, must be a non-empty string after
  trimming. A `""` or all-whitespace value fails.

It also checks that every `_meta.json` entry points at a real `en.json` key.

Both green = your translation is structurally correct. Ship it.

## How to add a NEW language

Adding, say, French (`fr`) is a copy plus two small registrations.

1. **Copy the catalog.** Duplicate `en.json` to
   `frontend/src/lib/i18n/locales/fr.json`. Translate the values (the loop above).
   Keep keys, placeholders, plural suffixes, and brand names intact.

2. **Register the locale in the engine** — `frontend/src/lib/i18n/index.ts`:
   - Import the catalog: `import fr from "./locales/fr.json";`
   - Add it to the `Locale` union type: `export type Locale = "en" | "es" | "fr";`
   - Add it to `LOCALES`: `["en", "es", "fr"]`
   - Add a label to `LOCALE_LABELS` — the language's **own** name, not the English
     name: `fr: "Français"`
   - Add it to `CATALOGS`: `{ en, es, fr }`

3. **Register it in the parity guard** — `frontend/src/lib/i18n/_parity.ts`. Add one
   line so the compiler type-checks your new catalog against `en.json`:

   ```ts
   import fr from "./locales/fr.json";
   const _frParity: Messages = fr;
   void _frParity;
   ```

Once registered, French shows up automatically in the sidebar language switcher (the
globe pill at the bottom of the sidebar) — there is no separate list to edit. Run the
two validators, then open a PR.

## What a parity failure looks like

So you recognize them when they happen.

**Compile failure (missing key)** — `pnpm --filter dlssync-frontend check`. You forgot
`common.dismiss` in `fr.json`:

```
src/lib/i18n/_parity.ts:5:7 - error TS2741: Property 'dismiss' is missing in type
'{ ... }' but required in type '{ updatesReady_one: string; ...; dismiss: string; ... }'.

5 const _frParity: Messages = fr;
        ~~~~~~~~~~
```

Fix: add the missing key (`"dismiss": "..."`) to your catalog, matching the English
key path.

**Test failure (extra key)** — `pnpm --filter dlssync-frontend test`. You typo'd
`common.dismis` instead of `common.dismiss`:

```
FAIL  tests/unit/i18nParity.test.ts > i18n catalog parity > fr.json has exactly the same keys as en.json
  AssertionError: extra keys in fr.json (not present in en.json)
  - Expected  []
  + Received  ["common.dismis"]
```

Fix: rename the bad key back to the English spelling. (You'll also see `common.dismiss`
reported as *missing*, because the real key is gone.)

**Test failure (empty value)** — same command. You left a value blank:

```
FAIL  tests/unit/i18nParity.test.ts > i18n catalog parity > every catalog value is a non-empty string
  AssertionError: fr.json: common.retry must be a non-empty string
  - Expected  true
  + Received  false
```

Fix: put a real translated string in. If you're not ready, paste the English value
back rather than leaving it empty.

## Reviewing a translation PR

If you're approving someone's language work, walk this list:

- [ ] Both validators are green: `pnpm --filter dlssync-frontend check` and
      `pnpm --filter dlssync-frontend test`.
- [ ] Keys are identical to `en.json` — no renames, no reorders that drop a key, no
      invented keys (the test catches missing/extra, but eyeball the diff anyway).
- [ ] Every `{placeholder}` from the English value is present in the translation,
      spelled the same.
- [ ] Plural keys carry the suffixes the language needs (`_one`/`_other` at minimum,
      plus `_few`/`_many`/etc. for languages that require them).
- [ ] No empty or whitespace-only values.
- [ ] Brand and product names left in English (NVIDIA, DLSS, Streamline, launcher
      names, preset letters…). Spot-check the `feature.*`, `group.*`, and `dlss.preset.*`
      sections — that's where brands cluster.
- [ ] For any ambiguous string, the translator consulted `_meta.json` and the result
      reads correctly in context (`where` tells you which screen to picture).
- [ ] If a new language was added, `index.ts` and `_parity.ts` were both updated.

## Pitfalls / FAQ

- **Don't translate brand or product names.** "DLSS" is not "Súper Muestreo de
  Aprendizaje Profundo." See the list near the top. When in doubt, the `_meta.json`
  `desc` line tells you which tokens to keep.
- **Don't reorder, rename, or remove keys.** The key path is how the app finds the
  string. Translating only changes the value on the right of the colon.
- **Keep `{placeholders}` verbatim.** Same braces, same spelling. You may move a
  placeholder to fit your grammar; you may not rename it. A renamed slot prints raw
  `{like_this}` to the user.
- **Never leave a value empty.** `""` fails the test. An untranslated-but-English value
  ships fine and just looks un-localized; an empty value breaks the check.
- **A source string is unclear or ambiguous?** Don't guess silently. Add a note to that
  key's `_meta.json` entry (extend the `desc`) describing the ambiguity, and mention it
  in your PR so a maintainer can clarify or reword the English source.
- **Why two checks and not one?** The compile check (`check`) catches *missing* keys
  with type errors; the Vitest test (`test`) additionally catches *extra* keys and
  *empty* values, which TypeScript can't see. Run both before you claim done.
