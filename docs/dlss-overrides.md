# DLSS preset and frame-generation overrides

DLSSync can force the DLSS Super Resolution model preset and the frame-generation mode for a game
without that game shipping native support, matching what the NVIDIA app exposes. This is
NVIDIA-only: Intel and AMD expose no equivalent per-application override.

## Scope: global and per-game

- Global — the Drivers tab → DLSS Overrides (next to the NVIDIA driver card). Writes the NVIDIA Base
  profile, applying to every application that does not have its own profile.
- Per-game — each game's detail drawer → DLSS Overrides. Writes that game's application profile,
  keyed to its detected executable. A per-game profile takes priority over the global one, the same
  precedence the NVIDIA driver enforces (Application profile > Current Global > Base).

## What can be set

- DLSS DLL override — force the game to use the latest installed Super Resolution or Frame
  Generation DLL.
- Super Resolution preset — `K`/`J`/`L`/`M` (transformer), the legacy CNN presets, or `Recommended`
  (the latest transformer model).
- Frame Generation mode — `Fixed` (a set multiplier) or `Dynamic` (DLSS 4.5, adjusts to a target
  frame rate).
- Fixed multiplier — `2×`, `3×` or `4×`.
- Dynamic target frame rate — used when the mode is `Dynamic`.

Each dropdown option shows a plain-language description of what it does and a "Learn more" link to the
canonical NVIDIA source, so the choice is explained inline.

## How it is applied

Overrides are written to the NVIDIA driver application profile through NVAPI DRS
(`NvAPI_DRS_SetSetting` / `NvAPI_DRS_SaveSettings`), the same mechanism the NVIDIA app and NVIDIA
Profile Inspector use. This is not DLL injection and not a kernel hook, and it needs no
administrator rights. The setting catalog (ids and value encodings) lives in the `nvapi-drs` crate;
the `nvapi64.dll` binding resolves functions through `nvapi_QueryInterface`.

`Reset to default` clears every override setting on the profile, restoring the driver default.

## Driver requirements

- DLSS 4 overrides require NVIDIA Game Ready Driver 572.16 or newer.
- Dynamic Multi Frame Generation (and 6×) requires Game Ready Driver 595.97 or newer on a GeForce
  RTX 50 series GPU; the panel disables the Dynamic option below that.
- Multiplayer titles with anti-cheat may treat a forced profile as tampering — the game drawer shows a
  ban-risk warning when anti-cheat is detected. See [anticheat.md](anticheat.md).
- Restart the game after applying an override for it to take effect.
