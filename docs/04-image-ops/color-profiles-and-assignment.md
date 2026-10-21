# Color Profiles and Assignment

- **Spec ID:** `IMG-006`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Assign Profile, Convert to Profile, working spaces, policies, rendering intents, and profile embedding predate CS6. (The underlying engine/model is specified in `ARCH-007`; this file is the document-lifecycle contract.)
- **Depends on:** `01-architecture/color-management.md` (`ARCH-007`), `01-architecture/file-formats.md` (`ARCH-011`), `01-architecture/document-model.md` (`ARCH-008`), `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `10-workflow-io/color-settings.md`, `10-workflow-io/save-and-save-as.md`, `10-workflow-io/printing.md`.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`. Numeric algorithm
> details (ICC transforms, rendering intents, BPC) live in `ARCH-007`; this file
> does not duplicate them.

## CS6 behavior

### Working spaces

- A **working space** is the intermediate space used to define and edit color in
  the application; each color model (RGB, CMYK, Gray, Spot) has one, chosen in
  `Edit > Color Settings`. Source: CS6 reference, "About color working spaces",
  "Working space options".
- A new document uses the working-space profile for its color mode. Working
  spaces also define the appearance of **untagged** documents.
- If an opened document is embedded with a profile that does **not match** the
  working space, a **color management policy** decides what happens; "in most
  cases, the default policy is to preserve the embedded profile."
- By default only Adobe-tested profiles appear in the working-space menus;
  `More Options` (Photoshop) reveals installed profiles. A profile must be
  bi-directional to appear.

### Three ways to change a document's profile

Source: CS6 reference, "Changing the color profile for a document".

| Operation | Menu | Effect on numbers | Effect on appearance |
|---|---|---|---|
| **Assign a new profile** | `Edit > Assign Profile` | Numbers unchanged | Appearance may change dramatically |
| **Remove the profile** | `Edit > Assign Profile` → Don't Color Manage | Numbers unchanged; document becomes untagged | Appearance now defined by the working space |
| **Convert colors** | `Edit > Convert To Profile` | Numbers shifted to preserve appearance | Appearance intended to be preserved |

`Assign Profile` options (Help verbatim intent):

- `Don't Color Manage This Document` — removes the profile; select "only if you
  are sure that you do not want to color-manage the document."
- `Working [color model: working space]` — assigns the working space profile.
- `Profile` — assigns a chosen profile without converting colors; "may
  dramatically change the appearance."

`Convert To Profile` options:

- `Destination Space` — the profile to convert to; the document is **converted
  and tagged** with it.
- `Conversion Options` — engine, rendering intent, black point compensation,
  dither.
- `Flatten Image` — flatten all layers onto one upon conversion.
- `Preview`.
- `Advanced` view exposes additional ICC profile types: **Multichannel**
  (more than four channels), **Device Link** (device-to-device without the PCS),
  and **Abstract** (custom effects; Lab/XYZ in and out). Basic view combines
  Gray/RGB/Lab/CMYK on one Profile menu.

### Missing and mismatched profiles

Source: CS6 reference, "About missing and mismatched color profiles", "Color
management policy options".

Exceptions the policy handles:

- A document/imported data with **no profile** (untagged).
- A document tagged with a profile **different from the working space**.

Policies (per RGB / CMYK / Gray):

- `Off` — ignore embedded profiles; do not assign the working space to new
  documents.
- `Preserve Embedded Profiles` — recommended; keep the embedded profile.
- `Convert To Working Space` — convert colors on open/import.
- `Preserve Numbers (Ignore Linked Profiles)` — InDesign/Illustrator CMYK only;
  not a Photoshop option.

Warning controls:

- `Profile Mismatches: Ask When Opening`, `Ask When Pasting`.
- `Missing Profiles: Ask When Opening`.
- **Profile warnings are turned off by default**; the user can turn them on to
  manage documents case-by-case.

On a warning, the Help's generic choices are:

- **Leave as-is** — use the embedded profile (mismatch), leave the document
  without a profile (missing), or preserve the numbers in pasted data.
- **Adjust** — for missing, assign the working space or a different profile; for
  mismatch, discard the profile or convert to the working space; for imports,
  convert to the working space to preserve appearance.

### Rendering intents and black point compensation

- Intent is selected when setting conversion options, soft-proofing, and
  printing. The default depends on the regional color setting (North America /
  Europe → **Relative Colorimetric**; Japan → **Perceptual**).
- The four intents (Perceptual, Saturation, Relative Colorimetric, Absolute
  Colorimetric) behave as specified in `ARCH-007`.
- `Use Black Point Compensation` "ensures that the shadow detail in the image is
  preserved by simulating the full dynamic range of the output device."
- `Use Dither` dithers colors when converting **8-bit-per-channel** images
  between color spaces to reduce banding.
- `Compensate For Scene-Rendered Profiles` compares video contrast when
  converting from scene to output profiles.

### How profiles embed in files

Source: CS6 reference, "Embed a color profile", "Preparing imported graphics for
color management".

- Formats that support embedded ICC profiles: **Adobe PDF, PSD (Photoshop),
  PSB/Large Document Format, AI, INDD, JPEG, Photoshop EPS, TIFF**.
- `Embed Color Profile` (Photoshop) / `ICC Profile` (Illustrator) is a per-save
  option; the exact name/location varies by format. `Save for Web` also offers
  convert-to-sRGB and include-profile options.
- In **PSD/PSB**, the raw ICC bytes live in image resource **1039** (`ICC
  Profile`); resource **1041** (`ICC Untagged Profile`) is a one-byte
  "intentionally untagged" flag.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Assign Profile` | Dialog | none documented | Don't-color-manage / Working space / Profile. |
| `Edit > Convert To Profile` | Dialog | none documented | Destination Space, Conversion Options, Flatten Image, Preview, Advanced. |
| `Edit > Color Settings` | Dialog | `Ctrl/Cmd+Shift+K` *(inferred)* | Working spaces, policies, warnings, engine/intent/BPC/dither, advanced, presets, `.csf`. |
| Profile-mismatch warning | Modal dialog (open/paste) | — | Use embedded / convert to working / discard. |
| Missing-profile warning | Modal dialog (open) | — | Assign working / assign different / leave untagged. |
| Document status bar | Drop-down read-out | — | Document profile name / `Untagged`. |
| `File > Save As` / `Save a Copy` | Checkbox | — | `Embed Color Profile` (per format). |
| `File > Save for Web` | Checkbox | `Ctrl+Shift+Alt+S` | Convert to sRGB; embed profile. |
| `File > Print` | Dialog section | `Ctrl+P` | Printer profile, rendering intent, BPC. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Assign Profile mode | radio | Working space *(inferred)* | Don't Color Manage / Working / Profile | Numbers never change. |
| Assign Profile picker | combo | current/working | installed ICC profiles | Appearance may change. |
| Convert To Profile destination | combo | working space *(inferred)* | installed profiles; Advanced adds Multichannel/Device Link/Abstract | Document is tagged with it. |
| Engine | combo | Adobe (ACE) | Adobe (ACE) (+ OS CMMs) | |
| Rendering Intent | combo | Relative Colorimetric (NA/EU) | Perceptual / Saturation / Relative / Absolute | |
| Black Point Compensation | bool | on *(inferred)* | on / off | Shadow-detail preservation. |
| Use Dither | bool | on *(inferred)* | on / off | 8-bit conversions only. |
| Flatten Image | bool | off | on / off | Convert To Profile only. |
| Preview | bool | off *(inferred)* | on / off | Convert To Profile. |
| Color Management Policy (RGB/CMYK/Gray) | combo | Preserve Embedded Profiles | Off / Preserve / Convert To Working | |
| Profile Mismatches: Ask When Opening / Pasting | bool pair | off | on / off | Warnings off by default. |
| Missing Profiles: Ask When Opening | bool | off | on / off | |
| Embed Color Profile | bool | on (format-dependent) | on / off | Per save. |

## Algorithms & pipeline

The transform engine, ICC model, intents, and BPC numerics are specified in
`ARCH-007` (`pictura_color`). This spec fixes only the **document operations**:

- **Assign** = replace the document's profile tag; do **not** touch pixel
  buffers.
- **Remove** = clear the profile tag and set the "intentionally untagged" flag.
- **Convert** = build `src_profile → dst_profile` transform (intent + optional
  BPC + optional dither), apply per pixel, then re-tag with the destination.
  Because it is lossy, undo must retain pre-conversion pixels.
- **Open policy** = choose assign / convert / preserve based on the policy plus
  any warning answer.
- **Display** = convert document → monitor profile (or proof chain), cached.

Adobe ACE's internal gamut mapping and BPC numerics are closed: **behavioural
parity only** (see `ARCH-007`).

## Rust module mapping

Proposed (reusing `pictura_color`):

- `pictura-core::color::assign_profile(doc) -> AssignProfileCommand`.
- `pictura-core::color::convert_profile(doc, dst, opts) -> ConvertProfileCommand`
  with `ConversionOptions { engine, intent, bpc, dither, flatten }`.
- `pictura-core::color::policy` — `ColorPolicy { Off, Preserve, ConvertToWorking }`
  and `resolve_open_profile(doc_profile, working, policy, warnings) -> OpenDecision`.
- `pictura-core::document::DocumentProfile` — `{ profile: Option<ColorProfileId>, raw_icc: Vec<u8>, untagged: bool }`.
- Boundary types: `ColorProfileId`, `ProfileBytes`, `Intent`, `ConversionOptions`.

## Qt6 component mapping

Per `ARCH-007`, referenced here rather than re-specified:

- `AssignProfileDialog`, `ConvertToProfileDialog`, `ColorSettingsDialog`,
  `ProfileMismatchWarningDialog`, `MissingProfileWarningDialog`,
  `DocumentProfileStatus` (status bar), `ColorProfileModel`.

The warning dialogs must be shown **before** the document is committed, so they
belong to the open/paste pipeline rather than the edit pipeline.

## Data-model impact

- `Document` carries `document_profile: Option<ColorProfileId>`, the retained raw
  ICC blob (for lossless PSD round-trip), and an `untagged_intent: bool`
  (resource 1041). See `ARCH-007`/`ARCH-008`.
- Assign/Remove are metadata-only single commands; Convert is a lossy pixel
  command whose undo stores pre-conversion pixels (`ARCH-009`).
- PSD/PSB resource **1039** = raw ICC profile; **1041** = untagged flag. Other
  formats carry ICC per their own specs (`ARCH-011`).
- On open with `Preserve`, the embedded profile is retained; on `Convert to
  Working`, the stored blob is replaced (or the converted values re-tagged).
- Soft-proof settings are document view state, not saved in PSD (`ARCH-007`).

## Edge cases

- **Untagged document**: display via the working space but keep untagged on save
  unless the user explicitly embeds a profile.
- **Indexed / Bitmap / Duotone**: no ICC working space (indexed has a color
  table; bitmap is 1-bit). Assign/Convert should be restricted or require a mode
  change (`04-image-ops/image-modes.md`, `04-image-ops/indexed-color.md`).
- **32-bit float**: transforms must not clamp intermediate values.
- **Absolute Colorimetric + BPC**: legal but semantically wrong for proofing; do
  not auto-enable BPC.
- **Very large / PSB**: convert by tile; never allocate a whole-document float
  buffer.
- **Missing monitor profile (Linux/Wayland)**: fall back to sRGB with a
  non-blocking status (`ARCH-007`).
- **Assign to a profile in a different color model** (e.g. RGB document, CMYK
  profile): CS6 lists only compatible profiles; guard.
- **Round-trip loss**: sRGB→Adobe RGB→sRGB is not bit-exact for out-of-gamut
  values.
- **Copy/paste across profiles**: applies `Ask When Pasting`; preserve numbers
  or convert.

## Parity acceptance criteria

1. Given an sRGB-tagged document, `Assign Profile` to Adobe RGB leaves every
   pixel byte unchanged and changes the displayed appearance; assigning sRGB
   again restores the original appearance exactly.
2. Given the same document, `Convert To Profile` to Adobe RGB changes pixel bytes
   and tags the document Adobe RGB; the in-gamut colors match an ACE reference
   within the tolerance in `11-cross-cutting/testing-strategy.md`.
3. Given `Don't Color Manage This Document`, the profile tag is removed and the
   document reports `Untagged`.
4. Given a PSD with an embedded ICC blob, opening and re-saving preserves the
   blob or re-embeds an equivalent profile; with `Embed Color Profile` off, no
   ICC resource is written.
5. Given policy `Preserve Embedded Profiles` and a mismatched document with
   `Ask When Opening` on, the warning appears and choices (use embedded /
   convert / discard) produce the same pixels as CS6 within tolerance.
6. Given an untagged document with `Missing Profiles: Ask When Opening` on, the
   missing-profile dialog offers assign-working / assign-other / leave-untagged.
7. Given `Convert To Profile` with `Flatten Image`, the result has a single
   layer.
8. Assign/Remove are single reversible undo steps with byte-identical pixels;
   Convert is a single undo step that restores the pre-conversion pixels
   exactly.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "Color settings": "About color working spaces",
  "Working space options", "About missing and mismatched color profiles", "Color
  management policy options", "Color conversion options", "About rendering
  intents", "Advanced controls in Photoshop"; "Working with color profiles":
  "About color profiles", "Embed a color profile", "Changing the color profile
  for a document", "Assign or remove a color profile (Illustrator, Photoshop)",
  "Convert document colors to another profile (Photoshop)", "Convert document
  colors to Multichannel, Device Link, or Abstract color profiles";
  "Color-managing imported images" and "Preparing imported graphics for color
  management" (formats that carry ICC).
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD image resources **1039** (ICC Profile) and **1041** (ICC Untagged
  Profile); color-mode and depth values.
- `https://www.color.org/icc_specs2.xalter` — ICC specification status (also
  cited by `ARCH-007`).
- `https://colormanagement.guide/en/workflows/photoshop-color-settings` — Color
  Settings anatomy, Assign vs Convert, `.csf`. Secondary/community source.

## Open questions

- **`Ctrl/Cmd+Shift+K` for Color Settings** is not confirmed from the CS6
  shortcut tables fetched here. *Resolves with:* the CS6 keyboard reference.
- **Defaults** for Assign Profile mode, Convert destination, engine/intent/BPC/
  dither/preview states. *Resolves with:* fresh-install CS6 captures.
- **Exact warning-dialog wording and button sets** in CS6 (versus the generic
  multi-app Help text). *Resolves with:* CS6 screenshots.
- **Whether Assign Profile offers rendering intents for Photoshop** (the Help
  lists intents only for the InDesign variant). *Resolves with:* CS6 dialog
  inspection.
- **PSD round-trip fidelity**: does CS6 rewrite the embedded ICC blob on save or
  preserve it byte-for-byte? *Resolves with:* inspecting CS6-made PSDs.
- **Convert To Profile availability** in Indexed/Duotone/Bitmap/32-bit.
  *Resolves with:* mode-by-mode CS6 inspection.
