# Color Settings

- **Spec ID:** `WF-011`
- **Status:** `Draft`
- **Parity tier:** `Core` (the dialog, policies, working spaces, `.csf` load/save, and profile-warning dialogs). The **Adobe (ACE)** engine and Perceptual/Saturation gamut mapping are behavioral parity only; see `ARCH-007`.
- **New in CS6:** `Changed` — CS6 adds **Blend Text Colors Using Gamma** (default **1.45**) for blending text layers in a gamma distinct from the document space, exposed under Color Settings → **More Options**. It also affects how CS6 text layers render in earlier Photoshop versions.
- **Depends on:** `ARCH-007` color-management (engine, transforms, lcms2), `ARCH-008` document-model, `ARCH-009` undo-history, `04-image-ops/color-profiles-and-assignment.md`, `04-image-ops/image-modes.md`, `10-workflow-io/file-info-and-metadata.md`, `10-workflow-io/printing.md`, `02-ui-ux/preferences.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Statements marked *(secondary)* are from community sources,
> not the CS6 Help PDF.

## CS6 behavior

**Edit > Color Settings** (`Ctrl+Shift+K` / `Cmd+Shift+K`) is the policy layer
for color management. It does **not** convert any image itself; it decides what
happens when documents are created, opened, imported, or converted. The CS6
Help PDF documents the dialog as an Adobe-suite feature ("Color settings",
"Working with color profiles") with Photoshop-specific items called out.

### Settings preset (top of dialog)

A **Settings** dropdown selects a tested configuration. Adobe ships regional
presets; UPDIG and community sources identify the prepress CMYK defaults:

| Preset | RGB working space (typical) | CMYK working space | Intent |
|---|---|---|---|
| North America General Purpose 2 | sRGB IEC61966-2.1 | U.S. Web Coated (SWOP) v2 | Relative Colorimetric |
| North America Prepress 2 | Adobe RGB (1998) | U.S. Web Coated (SWOP) v2 | Relative Colorimetric |
| North America Web/Internet | sRGB IEC61966-2.1 | — | — |
| Europe General Purpose 2 | sRGB IEC61966-2.1 *(secondary)* | Coated FOGRA27 (ISO 12647-2:2004) *(secondary)* | Relative Colorimetric *(secondary)* |
| Europe Prepress 2 | Adobe RGB (1998) *(secondary)* | Coated FOGRA27 (ISO 12647-2:2004) | Relative Colorimetric |
| Europe Web/Internet | sRGB IEC61966-2.1 *(secondary)* | — | — |
| Japan General Purpose 2 | sRGB *(secondary)* | Japan Color 2001 Coated *(secondary)* | Perceptual *(secondary)* |
| Japan Prepress 2 | Adobe RGB (1998) *(secondary)* | Japan Color 2001 Coated | **Perceptual** (Japan default) |
| Japan Web/Internet | sRGB *(secondary)* | — | — |
| Monitor Color | Monitor RGB  | — | — |
| Color Management Off | — | — | — |

- The PDF states **Relative Colorimetric** is the default intent for North
  America/Europe and **Perceptual** for Japan.
- The PDF does **not** enumerate the preset list; names beyond the
  North-America ones are *(secondary)*. Exact CS6 naming ("…2" vs "…") is an
  open question.
- **Monitor Color** makes no gamma adjustment; it is the default preview
  setting, and choosing it as an RGB working space effectively disables
  management *(secondary)*. **Color Management Off** ignores profiles.

### Working Spaces

Four rows, each a profile picker:

- **RGB** — recommended **Adobe RGB (1998)** for print or **sRGB
  IEC61966-2.1** for web/consumer cameras.
- **CMYK** — device-dependent, based on real ink/paper; Adobe supplies
  standard commercial-print profiles.
- **Gray** (Photoshop) — e.g. Gray Gamma 2.2; should pair with the RGB space's
  gamma.
- **Spot** (Photoshop) — dot-gain profile for spot channels and duotones
  (e.g. Dot Gain 20%).

By default only Adobe-tested profiles appear; selecting **More Options**
reveals all installed profiles (a profile must be **bi-directional** to appear).

### Color Management Policies

Per-mode (RGB, CMYK, Gray) policy for opening/importing color:

| Policy | Behavior |
|---|---|
| **Off** | Ignore embedded profiles; do not assign the working space to new docs |
| **Preserve Embedded Profiles** | Open/import in the embedded space (Adobe-recommended) |
| **Convert to Working …** | Convert incoming colors to the working space |

Warning checkboxes:

- **Profile Mismatches: Ask When Opening** — prompt when a document's profile
  differs from the working space.
- **Profile Mismatches: Ask When Pasting** — prompt on paste/drag-import.
- **Missing Profiles: Ask When Opening** — prompt for untagged documents.

Warnings are **off by default** in CS6.

### Mismatch / missing profile dialogs

- **Mismatch** (embedded profile ≠ working space): *Use the embedded profile*,
  *Convert to working space*, or *Discard the embedded profile*.
- **Missing** (untagged): *Assign working space*, *Assign then convert*, or
  *Leave without a profile*.

The recommended conservative path is to preserve data: use the embedded
profile / leave untagged, and only convert when the user explicitly wants the
working space.

### Conversion Options

- **Engine** — color management module; default **Adobe (ACE)**. (Apple CMM /
  Microsoft ICM exist on those platforms; Linux has no equivalent.)
- **Intent** — default rendering intent for conversions (see `ARCH-007`).
- **Use Black Point Compensation** — preserve shadow detail relative to the
  output device; recommended on.
- **Use Dither** — dither 8-bit conversions between spaces to reduce banding.

### Advanced controls (Photoshop: "More Options")

- **Desaturate Monitor Colors By** — 0–100% *(clamp secondary)*; preview aid
  for wide-gamut spaces; causes monitor/output mismatch.
- **Blend RGB Colors Using Gamma** — off by default; when on, RGB layers blend
  in the gamma space (1.00 = "colorimetrically correct", fewest edge
  artifacts).
- **Compensate for Scene-Rendered Profiles** — compares video contrast when
  converting scene→output profiles (After Effects default behavior).
- **Blend Text Colors Using Gamma** *(CS6 new)* — default **1.45**; blends
  text layers using a gamma distinct from the document space, matching other
  applications better. CS6 text layers look different in earlier Photoshop.

### Save / Load `.csf`

- **Save** writes the current settings as a **`.csf`** file; saving in the
  default Settings folder makes the name appear in the Settings dropdown.
- **Load** opens a `.csf` saved elsewhere. Settings are cross-application
  within the CS suite (shared `Creative Suite Color Settings`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Edit > Color Settings | Dialog | `Ctrl/Cmd+Shift+K` | Main dialog |
| Color Settings — Settings | Drop-down | — | Regional presets, custom, Monitor Color, Color Management Off |
| Color Settings — Working Spaces | 4 profile combos | — | RGB / CMYK / Gray / Spot |
| Color Settings — Policies | 3 combos + 3 checks | — | RGB / CMYK / Gray; mismatch/missing warnings |
| Color Settings — Conversion Options | Combos + checks | — | Engine, Intent, BPC, Dither |
| Color Settings — Advanced | Checks/slider | — | More Options reveals |
| Color Settings — Save / Load | Buttons | — | `.csf` files |
| Edit > Assign Profile | Dialog | — | Reinterpret numbers (see `ARCH-007`) |
| Edit > Convert to Profile | Dialog | — | Transform numbers |
| Bridge > Edit > Creative Suite Color Settings | Menu | — | Suite-wide sync |
| View > Proof Setup / Proof Colors | Menu/toggle | `Ctrl+Y` | See `ARCH-007`, `WF-013` |

## Parameters & ranges

| Control | Type | Default (CS6) | Range / options | Notes |
|---|---|---|---|---|
| Settings preset | Enum | North America General Purpose 2 *(secondary)* | Regional presets, Monitor Color, Color Management Off, custom | PDF lists no exact default |
| RGB working space | Profile | sRGB IEC61966-2.1 | sRGB, Adobe RGB (1998), ProPhoto RGB, Apple RGB, ColorMatch RGB, Monitor RGB, custom | More Options reveals more |
| CMYK working space | Profile | U.S. Web Coated (SWOP) v2 | Regional CMYK profiles, custom | Device-dependent |
| Gray working space | Profile | Gray Gamma 2.2 *(secondary)* | Gray Gamma 1.8/2.2, sGray, custom | |
| Spot working space | Profile | Dot Gain 20% *(secondary)* | Dot-gain %, custom | |
| Policy RGB/CMYK/Gray | Enum | Preserve Embedded Profiles | Off / Preserve / Convert to Working | |
| Ask When Opening (mismatch) | Bool | Off | on/off | |
| Ask When Pasting (mismatch) | Bool | Off | on/off | |
| Ask When Opening (missing) | Bool | Off | on/off | |
| Engine | Enum | Adobe (ACE) | Adobe (ACE), platform CMMs | Linux: ACE unavailable |
| Intent | Enum | Relative Colorimetric (NA/EU), Perceptual (Japan) | Perceptual / Relative / Saturation / Absolute | |
| Use Black Point Compensation | Bool | On *(secondary)* | on/off | |
| Use Dither | Bool | On *(secondary)* | on/off | 8-bit only |
| Compensate for Scene-Rendered Profiles | Bool | Off | on/off | Advanced |
| Desaturate Monitor Colors By | Percent | 0% | 0–100% *(unverified)* | Advanced |
| Blend RGB Colors Using Gamma | Bool + float | Off; 1.00 | ~1.00–2.20 *(unverified)* | Advanced |
| Blend Text Colors Using Gamma | Bool + float | On/1.45 *(CS6 per What's new)* | ~1.45 default | Advanced; CS6 new |

## Algorithms & pipeline

This document owns the **dialog semantics**; the transform engine, profile
model, rendering intents, BPC, and embedding are specified in `ARCH-007`. The
Color Settings→engine flow is:

1. **Load `.csf`** (or built-in preset) → `ColorSettings` struct.
2. **On File > New** — assign the working-space profile for the document's
   color mode.
3. **On open/import** — apply the per-mode policy:
   - *Off*: strip/ignore profile; no assignment.
   - *Preserve*: keep embedded profile; if missing, leave untagged (assume
     working space for display only).
   - *Convert*: build source→working transform and convert.
   Warn first if the matching "Ask" option is set.
4. **On conversion** — build `Transform` with engine + intent + BPC (+dither
   for 8-bit destinations).
5. **Display** — document→monitor transform from the monitor profile.
6. **Blending** — when `Blend RGB Colors Using Gamma` is on, composite RGB in
   the chosen gamma; text layers use `Blend Text Colors Using Gamma` (1.45).

ACE's internal gamut mapping, dither kernel, and BPC numerics are closed:
**behavioral parity only, algorithm TBD**. `.csf` is a documented-enough
container for the settings struct; its exact binary layout is not in the CS6
Help PDF (open question).

## Rust module mapping

Proposals (the engine itself lives in `ARCH-007`):

- `pictura_color::settings` — `ColorSettings { preset_id, working_rgb,
  working_cmyk, working_gray, working_spot, policies: [Policy;3],
  ask_mismatch_open, ask_mismatch_paste, ask_missing_open, engine, intent, bpc,
  dither, compensate_scene, desaturate_monitor, blend_rgb_gamma,
  blend_text_gamma }`.
- `pictura_color::settings::csf` — `load_csf(path) -> ColorSettings`,
  `save_csf(&ColorSettings, path)`; built-in preset table with regional
  defaults and profile IDs.
- `pictura_color::policy` — `Policy { Off, Preserve, Convert }`, plus
  `decide(embedded: Option<ProfileId>, working: ProfileId) -> OpenDecision`
  and the warning-prompt enum.
- `pictura_color::engine` — `Transform` factory honouring intent/BPC/dither
  (wraps `lcms2`, see `ARCH-007`).
- `pictura_color::blend` — gamma-space RGB blending and text-gamma (1.45)
  blending used by the compositor.

Crossing types: `ColorSettings`, `Policy`, `OpenDecision`, `ProfileId`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorSettingsDialog` | `QDialog` | Full dialog: preset combo, working-space combos, policy combos + warning checks, conversion options, advanced group, Save/Load |
| `SettingsPresetCombo` | `QComboBox` | Built-in regional presets + custom; reflects loaded `.csf` |
| `ProfileCombo` | `QComboBox` | Profile picker backed by `ColorProfileModel`; hides non-bi-directional profiles unless More Options |
| `PolicyCombo` | `QComboBox` | Off / Preserve / Convert to Working per mode |
| `CsFileDialog` | `QFileDialog` | `.csf` load/save with default Settings folder |
| `ProfileMismatchDialog` | `QMessageBox`-style | Use embedded / Convert / Discard |
| `MissingProfileDialog` | `QDialog` | Assign / Assign+convert / Leave untagged |
| `ColorProfileModel` | `QAbstractListModel` | Cached profile metadata, shared with `ARCH-007` widgets |

Widgets over QML: dense, keyboard-navigable, consistent with `ARCH-003`.

## Data-model impact

- `ColorSettings` is **application-level** (Preferences), not per document; a
  loaded `.csf` replaces the current global settings and is persisted in the
  preference store (`11-cross-cutting/preference-storage.md`).
- Documents still carry their own `document_profile` and `untagged_intent`
  (`ARCH-007`, PSD resources 1039/1041); Color Settings never edits them.
- Opening a document may record an `OpenDecision` in the document's provenance
  (optional), but the policy itself is not serialized into PSD.
- **Undo:** Color Settings changes are global preferences and are **not**
  Photoshop history states (per CS6 "changes to color settings … are not
  reflected in the History panel"). Assign/Convert (which are document edits)
  remain undoable via `ARCH-007`.

## Edge cases

- **Untagged documents** — display with the working space, but keep untagged;
  do not silently tag on save.
- **Monitor RGB / Monitor Color** — effectively disables management; warn that
  it defeats color consistency.
- **Missing monitor profile (Linux/Wayland)** — fall back to sRGB and surface a
  non-blocking status.
- **ACE unavailable on Linux** — the Engine menu must show the Linux engine
  (lcms2/OpenICC) rather than a broken "Adobe (ACE)" entry; parity of engine
  naming is not literal.
- **Non-bi-directional profile** — hidden unless More Options; if selected,
  reject with a clear error.
- **`.csf` referencing an absent profile** — load with a placeholder and prompt
  to substitute; never crash.
- **Japan Perceptual default** — regional presets differ; do not hard-code
  Relative Colorimetric globally.
- **Text gamma migration** — opening a CS6 text layer in an older engine must
  be a known difference (CS6 text looks different in earlier versions).
- **Cross-application suite sync** — editing via Bridge should propagate; on
  Linux, only Kooka Pictura exists.
- **8/16/32-bit** — dither applies only to 8-bit conversions.

## Parity acceptance criteria

- Given the default settings, opening an Adobe RGB file with policy Preserve
  leaves pixel values unchanged and reports the embedded profile.
- Given policy Convert to Working RGB, the document is converted to the working
  space and the operation is recorded as one undoable command.
- Given `Ask When Opening` on and a profile mismatch, the mismatch dialog
  appears with the three correct choices.
- Given a missing profile with its warning on, the missing-profile dialog
  offers assign / assign-and-convert / leave untagged.
- Given a custom configuration, Save writes a `.csf`; Load restores it exactly
  (all working spaces, policies, conversion options, advanced values).
- Given `Blend Text Colors Using Gamma` at 1.45, a text layer over a colored
  background blends identically to a CS6 reference within the tolerance in
  `11-cross-cutting/testing-strategy.md`.
- Given `Blend RGB Colors Using Gamma` off, RGB compositing matches the
  document-space reference; on with gamma 1.00, edge artifacts decrease.
- Given Japan regional settings, the default intent is Perceptual; given
  North America/Europe, Relative Colorimetric.
- Given Monitor Color selected, distinct colors may collapse and the dialog
  warns/indicates management is off.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  CS6 Help corpus (`pdftotext -layout`). Established: Color Settings sections
  (Customize, Working Spaces, Missing/Mismatched Profiles, Policies, Conversion
  Options, Rendering Intents, Advanced Controls); policy option definitions
  (Off / Preserve Embedded / Convert to Working), warning options; conversion
  options (Engine, Intent, BPC, Dither, Compensate For Scene-Rendered
  Profiles); Working Spaces RGB/CMYK/Gray/Spot descriptions; intent defaults
  (Relative Colorimetric NA/EU, Perceptual Japan); `More Options`; `.csf`
  Save/Load and default-location behavior; **Blend Text Colors Using Gamma**
  CS6 addition with default **1.45**; Blend RGB Colors Using Gamma;
  Desaturate Monitor Colors By; "changes to color settings are not reflected in
  the History panel".
- `https://colormanagement.guide/en/workflows/photoshop-color-settings/` —
  dialog anatomy, policy meanings, Assign vs Convert, built-in presets
  (North America General Purpose 2 / Prepress 2 / Web/Internet), `.csf`
  sharing, Monitor RGB disabling management. Secondary/community; used for the
  parts the PDF does not enumerate.
- `http://www.updig.org/guidelines/ir_icc_profiles.html` — regional prepress
  CMYK defaults: U.S. Web Coated (SWOP) v2 = North American Prepress 2; Coated
  FOGRA27 (ISO 12647-2-2004) = Europe Prepress 2; Japan Color 2001 Coated =
  Japan Prepress 2. Secondary/industry guideline.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD resource 1039/1041 profile embedding and resource 1062 print scale
  (used for the data-model cross-reference).

Discovery searches (results, not opened as pages): searches for the full CS6
regional preset list and "Monitor Color" / "Color Management Off"; a Smashing
Magazine pre-press article corroborating the North America / Europe / Japan
prepress entry points was seen in results but not fetched.

Not fetched / not used: `helpx.adobe.com` (403). The `.csf` binary layout is
**not** documented in any fetched source.

## Open questions

- **Exact CS6 preset list and names.** Whether CS6 uses "North America
  Prepress 2", "Europe Prepress 2", "Japan Prepress 2", "Monitor Color",
  "Color Management Off", and which custom defaults ship. *Resolves with:* a
  CS6 dialog capture or an archived CS6 Help page.
- **Default working spaces per preset in CS6.** RGB=Adobe RGB for prepress and
  sRGB for general/web is consistent across sources, but Gray/Spot defaults and
  Europe/Japan RGB values are *(secondary)*. *Resolves with:* a CS6 capture.
- **`.csf` file format.** Binary layout, versioning, profile references.
  *Resolves with:* hex analysis of CS6-created `.csf` files.
- **Desaturate / Blend RGB gamma clamps.** PDF gives no values.
  *Resolves with:* CS6 UI observation.
- **Blend Text Colors Using Gamma default state.** The What's-new text implies
  the setting exists with default 1.45; whether it is enabled by default is
  unverified. *Resolves with:* a CS6 capture.
- **Engine parity target on Linux.** Whether Kooka Pictura exposes an "Adobe
  (ACE)" label (misleading) or a Linux engine name. *Resolves with:* a product
  decision in `OVR-003`/`ARCH-007`.
- **Suite-wide sync scope.** Whether Kooka Pictura participates in any
  cross-application settings mechanism (none on Linux). *Resolves with:* a
  `OVR-003` decision.
