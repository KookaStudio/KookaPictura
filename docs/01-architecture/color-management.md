# Color Management

- **Spec ID:** `ARCH-007` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` (the Color Settings model, Adobe ACE engine, ICC v4 profile support, and soft proofing predate CS6; CS6 adds no documented color-management feature over CS5 — inferred, see `## Open questions`)
- **Depends on:** `ARCH-008` document-model, `ARCH-011` file-formats, `04-image-ops/color-profiles-and-assignment.md`, `04-image-ops/bit-depth-and-conversion.md`, `04-image-ops/image-modes.md`, `10-workflow-io/color-settings.md`, `10-workflow-io/printing.md`

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

Photoshop CS6 is fully color-managed. It distinguishes three roles for a profile:

- **Source / document profile** — the profile embedded in (or assigned to) the
  open document. Shown at the bottom of the document window; `Untagged` means no
  profile is embedded.
- **Monitor profile** — the OS default display profile, used to convert source
  colors to the display on the fly ("monitor compensation").
- **Destination / output profile** — the profile a conversion targets, e.g. a
  printer-paper-ink profile during print or soft proofing.

Profiles are attached to documents in two distinct ways:

- **Edit > Assign Profile** reinterprets the existing numeric values. Pixel bytes
  are not changed; the meaning of the numbers changes, so the image appearance
  changes. It is used to correct a missing or wrong tag.
- **Edit > Convert to Profile** transforms pixel values from the current source
  profile to a chosen destination profile, attempting to preserve appearance.
  Values change; out-of-gamut colors may be clipped or compressed and cannot be
  recovered.

Working spaces and policies are set once in **Edit > Color Settings**
(`Ctrl+Shift+K` on Windows, `Cmd+Shift+K` on macOS). The dialog selects the
default RGB, CMYK, Gray, and Spot working spaces, the color-management policy for
incoming files, the conversion engine and default rendering intent, and advanced
display options. The result can be saved as a `.csf` settings file.

Policy per color mode (RGB, CMYK, Gray) is one of:

- **Off** — embedded profiles are ignored; no conversion.
- **Preserve Embedded Profiles** — documents open in their embedded space
  regardless of the working space.
- **Convert to Working …** — incoming documents are converted to the working
  space on open.

When policy is "Preserve" and a mismatch or missing profile occurs, CS6 can show
a warning dialog offering: use the embedded profile, convert to the working
space, or discard the profile (missing-profile case: assign working space, assign
then convert, or leave untagged). The recommended professional setting is
Preserve Embedded Profiles with mismatch warnings enabled.

### Soft proofing

**View > Proof Setup** selects a device to simulate, a rendering intent, black
point compensation, and optional paper/ink simulation:

- **Custom…** opens the proof setup dialog (`Device to Simulate`, `Preserve RGB
  Numbers` / `Preserve CMYK Numbers`, `Rendering Intent`, `Black Point
  Compensation`, `Simulate Paper Color`, `Simulate Black Ink`).
- Built-in entries include `Working CMYK`, `Working Gray`, `Monitor RGB`, and
  `Macintosh RGB` / `Internet Standard RGB` (inferred naming).
- **View > Proof Colors** (`Ctrl+Y`) toggles proofing on and off; the document
  title bar appends the proof space name while active.
- **View > Gamut Warning** (`Ctrl+Shift+Y`) paints out-of-gamut pixels a flat
  warning color (default gray) chosen in Preferences.

### Bit depth and gamut

Color transforms are defined at 8, 16, and 32 bits per channel. 32-bit mode is
floating-point and used by HDR workflows; some adjustments and filters are
restricted to 8/16-bit (inferred). Gamut is the set of colors a space can
represent: sRGB is small, Adobe RGB is wider (notably cyans/greens), ProPhoto RGB
is very wide and contains imaginary colors. Converting a large working space to a
small one discards out-of-gamut information permanently once saved.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Edit > Color Settings | Dialog | `Ctrl/Cmd+Shift+K` | Working spaces, policies, conversion options, advanced controls, presets, `.csf` save/load |
| Edit > Assign Profile | Dialog + submenu | n/a | Lists profiles; options: don't color manage, working RGB/CMYK/Gray, specific profile |
| Edit > Convert to Profile | Dialog | n/a | Source Space read-out, Destination Space, Conversion Options (engine, intent, BPC, dither), Preview |
| View > Proof Setup | Submenu + dialog | n/a | Custom, Working CMYK/Gray, Monitor RGB, Internet/Macintosh RGB; Choose proof; proof toggle |
| View > Proof Colors | Toggle | `Ctrl+Y` | Title bar shows proof space |
| View > Gamut Warning | Toggle | `Ctrl+Shift+Y` | Warning color set in Preferences > Transparency & Gamut |
| Document status bar | Drop-down | n/a | "Document Profile" read-out; "Untagged" when no profile |
| File > Save As / Save a Copy | Dialog | n/a | "Embed Color Profile" checkbox per format |
| File > Print | Dialog | `Ctrl+P` | Color Handling, Printer Profile, Rendering Intent, Black Point Compensation |
| File > Save for Web | Dialog | `Ctrl+Shift+Alt+S` | "Convert to sRGB" and "Embed Color Profile" checkboxes, preview in Monitor Color |
| Preferences > Transparency & Gamut | Pane | n/a | Gamut Warning color and opacity |
| View > Proof Setup > Custom | Dialog | n/a | Device to Simulate, intent, BPC, Simulate Paper Color / Black Ink |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Working Space RGB | enum (profile) | sRGB IEC61966-2.1 | sRGB, Adobe RGB (1998), ProPhoto RGB, Apple RGB, ColorMatch RGB, Monitor RGB, custom `.icc` | "Monitor RGB" is a behavior; selecting it effectively disables management |
| Working Space CMYK | enum (profile) | U.S. Web Coated (SWOP) v2 | Coated/Uncoated stock presets, custom | Print-oriented |
| Working Space Gray | enum (profile) | Gray Gamma 2.2 | Gray Gamma 1.8, 2.2, sGray, custom | Should pair with Gamma of the RGB space |
| Working Space Spot | enum (profile) | Dot Gain 20% | Dot-gain percentages, custom | Spot/duotone separation |
| Color Management Policy (RGB/CMYK/Gray) | enum | Preserve Embedded Profiles | Off / Preserve / Convert to Working | Per mode |
| Mismatch / Missing Profile warnings | bool pair | Off | on / off | `Ask When Opening`, `Ask When Pasting` |
| Engine | enum | Adobe (ACE) | Adobe (ACE), Apple CMM (macOS), Microsoft ICM (Windows) | CS6 exposes ACE |
| Rendering Intent | enum | Relative Colorimetric | Perceptual, Relative Colorimetric, Saturation, Absolute Colorimetric | Default for conversions |
| Black Point Compensation | bool | On | on / off | See algorithm below |
| Use Dither | bool | On | on / off | 8-bit conversion noise to mask banding |
| Compensate for Scene-referred Profiles | bool | Off | on / off | Advanced |
| Desaturate Monitor Colors By | percent | 0% | 0–40% (inferred) | Advanced preview aid |
| Blend RGB Colors Using Gamma | float | 1.00 | ~1.00, 1.45, 2.20 (inferred) | Advanced; linear blending |
| Simulate Paper Color | bool | Off | on / off | Proof setup |
| Simulate Black Ink | bool | Off | on / off | Proof setup |
| Embed Color Profile | bool | On (format-dependent) | on / off | Per save |

All ranges not explicitly marked are the options listed in the CS6 dialogs;
numeric clamps for the two Advanced controls are *(inferred)*.

## Algorithms & pipeline

### Profile model

An ICC profile describes a mapping between a device/space encoding and the
Profile Connection Space (PCS). ICC.1:2022 (profile version 4.4.0.0; ISO
15076-1:2005) is the current specification; v2 profiles remain valid and a v4
system must process them. Profiles use either matrix/TRC tags (typical for RGB)
or LUT-based tags (`A2B0`/`B2A0` and relatives, typical for CMYK/print). A
transform is built from two profiles through the PCS.

### Transform and rendering intents

A conversion builds a `Transform`, then applies it per pixel. The four ICC
rendering intents:

| Intent | Behavior | Typical use |
|---|---|---|
| Perceptual | Compresses/gamut-maps the whole source range into the destination gamut; colors at the gamut edge are separated to preserve detail | Photographs to print |
| Relative Colorimetric | Maps source white to destination white (relative), reproduces in-gamut colors exactly, clips out-of-gamut colors | Photographs, conversions between spaces; paired with BPC |
| Saturation | Expands saturated colors toward the gamut edge | Vector graphics, logos |
| Absolute Colorimetric | No white-point adaptation; simulates source including paper white and black | Hard proofing of a press on another device |

Perceptual and Saturation transforms are vendor-specific: ICC does not fully
specify the gamut mapping, so output differs between CMMs. Behavioral parity must
therefore be tolerant, not bit-exact.

### Black point compensation

BPC maps the source black point to the destination's black point (its darkest
reproducible value, e.g. a matte paper's Dmax) and scales the tonal range rather
than clipping it, preserving shadow separation. It is meaningful for
Relative Colorimetric output to a printer profile; it has no effect for
Perceptual (already gamut-compressed) and should be off for Absolute
Colorimetric proofing. It is not applicable to matrix-based conversions between
working RGB spaces.

Proposed transform stages:

1. Decode source samples to a working linear/float representation at the
   document bit depth.
2. Apply source profile → PCS.
3. Apply intent + optional BPC.
4. Apply PCS → destination profile.
5. Re-encode at the destination bit depth; apply ordered/error-diffusion dither
   for 8-bit destinations when `Use Dither` is on.
6. Tag the destination.

*(inferred)* The exact ACE internal gamut-mapping, dither kernel, and BPC
numerics are closed; parity is behavioral.

### Embedding and tagging

- PSD/PSB store the raw ICC profile bytes in image resource **1039** (`ICC
  Profile`); resource **1041** (`ICC Untagged Profile`) is a one-byte flag
  meaning "intentionally untagged, do not assume a profile on open".
- TIFF/JPEG/PNG/PDF and others carry ICC/EXIF/IPTC/XMP per their own
  specifications (see `ARCH-011`).
- When a document has no profile, Photoshop assumes the working space for
  display/conversion, but marks it untagged (inferred from the untagged flag
  semantics).

### Display path

Canvas rendering converts document colors to the monitor profile through the
same transform machinery, typically cached when the source space and monitor
profile are unchanged. Soft proofing composes a document→proof-space transform
with a proof-space→monitor transform, and (optionally) paper/black-ink
simulation. A GPU path should precompute a 1D/3D LUT or a matrix+curve and
sample it in the shader; the CPU reference path uses lcms2 directly.

## Rust module mapping

Proposals. The color engine wraps Little CMS through the `lcms2` crate, which is
a safe wrapper over the C library and exposes `Profile`, `Transform`, `Intent`,
`ColorSpaceSignature`, `ThreadContext`, `Flags`, `CIEXYZ`/`CIELab` types, and
`Pod`/`Zeroable` pixel constraints. `moxcms` (used by the `image` crate) is a
possible pure-Rust fallback for basic transforms but does not replace a full CMS.

- `pictura_color::profile` — `ColorProfile` newtype owning an `lcms2::Profile`;
  load from bytes/in-memory, query class, color space, PCS, version, tags. Raw
  bytes retained for PSD resource 1039 round-tripping.
- `pictura_color::space` — `WorkingSpace { Rgb, Cmyk, Gray, Spot, Lab, Xyz }` and
  named built-ins (sRGB, Adobe RGB, ProPhoto, Gray Gamma 2.2, Dot Gain). Built-in
  profiles are generated or loaded from a bundled profile set.
- `pictura_color::intent` — `Intent { Perceptual, Relative, Saturation, Absolute }`
  mirroring `lcms2::Intent`; plus `ConversionOptions { intent, bpc, dither }`.
- `pictura_color::transform` — `ColorTransform` wrapping `lcms2::Transform`;
  `apply_u8`, `apply_u16`, `apply_f32` specializations; cached by
  `(src_profile_id, dst_profile_id, intent, bpc)`. `ThreadContext` per worker
  thread; `Flags::NO_CACHE` when a transform is shared read-only across threads
  (the crate documents this interaction).
- `pictura_color::proof` — `ProofSetup { device, intent, bpc, paper_color,
  black_ink }`; builds composed transforms and gamut-mask test.
- `pictura_color::gamut` — out-of-gamut predicate (round-trip through destination
  and compare ΔE / per-channel delta).
- `pictura_color::settings` — `ColorSettings { working_spaces, policies,
  engine, default_intent, bpc, dither, advanced }`; `.csf` load/save is a
  separate concern (see `10-workflow-io/color-settings.md`).
- `pictura_color::embed` — helpers to read/write ICC payloads for PSD resource
  1039/1041 and per-format containers.

Pixel types crossing module boundaries: `&[u8]`, `&[u16]`, `&[f32]` planar or
interleaved plus a `PixelLayout` descriptor; `Transform` is the only opaque
handle.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorSettingsDialog` | `QDialog` | Working spaces (RGB/CMYK/Gray/Spot combos), policies, warnings, conversion options, advanced group, preset combo, Save/Load `.csf` |
| `AssignProfileDialog` | `QDialog` | Don't-color-manage radio, working-space radio, profile picker with preview |
| `ConvertToProfileDialog` | `QDialog` | Source read-out, destination picker, intent/BPC/dither, Preview toggle |
| `ProofSetupDialog` | `QDialog` | Device-to-simulate, Preserve Numbers, intent, BPC, paper/black-ink simulation |
| `ProfileCombo` | `QComboBox` | Discovers installed profiles via the platform profile store; caches metadata |
| `DocumentProfileStatus` | `QWidget` (status bar) | Shows document profile / `Untagged` / proof space; opens the picker |
| `ColorProfileModel` | `QAbstractListModel` | Profile list for combos and the status bar |
| `GamutWarningOverlay` | Canvas overlay widget | Paints out-of-gamut pixels with the warning color; toggled by `Ctrl+Shift+Y` |

Widgets are chosen over QML for consistency with the shell decision in
`ARCH-003`. Profile discovery on Linux must probe both `~/.local/share/icc` /
`/usr/share/color/icc` (Compatible with ICC/`colord`) and, where available, the
`colord` D-Bus service; monitor profile retrieval is a separate concern owned by
the platform-integration layer.

## Data-model impact

- `Document` carries `document_profile: Option<ColorProfileId>` and an
  `untagged_intent: bool` (PSD resource 1041). The raw profile blob is retained
  for lossless PSD round-trip even if it is not the active working space.
- Layer/channel pixel buffers are stored in the document color mode and bit
  depth; the document profile is metadata, not a per-pixel transform at rest.
- `DocumentColorMode` and `DocumentBitDepth` are document fields
  (`ARCH-008`). Changing either is a destructive command with its own undo
  record.
- Proof setup is document-level view state in CS6 (not saved in PSD) plus a
  global default in Preferences; represent both, persist only the global
  default.
- Undo records: Assign Profile and Convert to Profile are single commands.
  Conversion stores either the before-image or a reversible transform
  descriptor; because conversion is lossy, undo stores the pre-conversion pixel
  data (see `ARCH-009`).
- XMP can carry profile-related metadata (e.g. `crs:` raw settings); do not
  assume the embedded ICC blob and XMP agree.

## Edge cases

- **Untagged documents** — assume no profile; Photoshop displays using the
  working space but keeps the file untagged unless the user saves with a
  profile. Do not silently tag on save.
- **Missing monitor profile** — on Linux/Wayland a compositor may not expose an
  ICC profile. Fall back to sRGB and surface a non-blocking status.
- **8 vs 16 vs 32-bit** — transforms must not clamp intermediate values in
  32-bit float; 8-bit conversions use dither to avoid banding. 16-bit is the
  minimum for wide-gamut spaces (ProPhoto at 8-bit bands badly).
- **CMYK and Lab documents** — profiles are LUT-based; black generation,
  total-ink limits, and dot gain are profile concerns, not app logic.
- **Indexed and Bitmap modes** — indexed has a color table, not an ICC working
  space; bitmap (1-bit) has no meaningful per-pixel transform. Restrict
  conversion commands or convert mode first.
- **Absolute Colorimetric + BPC** — legal but semantically wrong for proofing;
  do not auto-enable BPC for Absolute.
- **Large/PSB documents** — transforms must stream by tile; avoid allocating a
  full-document float buffer.
- **Thread safety** — an `lcms2::Transform` is not concurrently usable unless
  created with `Flags::NO_CACHE`; otherwise use one transform per thread.
- **Rounding** — GPU LUT sampling and CPU lcms2 will differ by small deltas;
  parity tolerance must allow this.

## Parity acceptance criteria

- Given a document with an embedded sRGB profile and a working space of Adobe
  RGB, opening it with policy "Preserve" leaves the pixel values unchanged and
  reports the sRGB profile.
- Given the same document and policy "Convert to Working", the result matches a
  reference ACE conversion to within the tolerance in
  `11-cross-cutting/testing-strategy.md` (per-channel delta TBD) and is tagged
  Adobe RGB.
- Given an sRGB image assigned Adobe RGB, pixel bytes are unchanged and the
  displayed appearance changes; assigning sRGB again restores the original
  appearance (no data loss).
- Given Convert to Profile sRGB→Adobe RGB→sRGB, the round trip is within the
  defined ΔE tolerance for in-gamut colors and shadow separation is preserved
  with BPC on.
- Given Relative Colorimetric with BPC on, converting a step wedge to a printer
  profile preserves the number of distinguishable dark steps; with BPC off the
  darkest steps collapse.
- Given an out-of-gamut test image and Gamut Warning on, the set of flagged
  pixels matches the reference warning mask within the defined coverage
  tolerance.
- Given Proof Colors on and the title bar, the document title shows the proof
  space name and toggling `Ctrl+Y` restores the unproofed display.
- Given a 16-bit ProPhoto document, an 8-bit conversion does not produce
  banding beyond the defined threshold on a smooth gradient.
- Given `Embed Color Profile` off, saving does not write an ICC blob; given on,
  the exact source profile bytes are recoverable from the saved file.

## Sources

Fetched for this document:

- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification: image resource IDs 1039 (ICC
  Profile), 1041 (ICC Untagged Profile), 1028 (IPTC-NAA), 1058/1059 (EXIF),
  1060 (XMP); PSD color modes and bit depths. Establishes the PSD-side
  embedding contract.
- `https://www.color.org/icc_specs2.xalter` — ICC specification status:
  ICC.1:2022 (profile version 4.4.0.0), ISO 15076-1:2005, v2 backwards
  compatibility, iccMAX ICC.2:2023. Establishes the ICC version contract.
- `https://colormanagement.guide/en/workflows/photoshop-color-settings` —
  Color Settings dialog anatomy (Working Spaces, Policies, Conversion Options,
  Advanced), Assign vs Convert, presets, `.csf` files, recommended defaults.
  Secondary/community source.
- `https://colormanagement.guide/en/icc-profiles/black-point-compensation` —
  BPC definition, tonal-range scaling, intent interactions. Secondary/community
  source.
- `https://www.colourphil.co.uk/rendering_intents.shtml` — four rendering
  intents, relative/absolute/perceptual/saturation behavior, BPC placement.
  Secondary/community source.
- `https://gballard.net/psd/cmstheory.html` — source/monitor/destination
  profile roles, embedded vs untagged handling, soft proofing and gamut warning
  concepts. Secondary/community source.
- `https://docs.rs/lcms2/latest/lcms2/` — `lcms2` 6.2.0 API surface: `Profile`,
  `Transform`, `Intent`, `Flags`, `ThreadContext`, pixel `Pod`/`Zeroable`,
  colorimetry types. Establishes the Rust color-engine candidate.
- `https://docs.rs/image/latest/image/` and
  `https://docs.rs/image/latest/image/codecs/index.html` — `image` 0.25.10
  delegates CMS work to `moxcms`; confirms a pure-Rust CMS alternative.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help. Fetch attempt exceeded the 5 MB response limit; not parsed
  in this pass (see `## Open questions`). Provided by the project as an
  authoritative source.

Not fetched / not used as a source: `helpx.adobe.com` pages (documented as HTTP
403 in `README.md`).

## Open questions

- **Exact CS6 Color Settings ranges.** The clamps for "Desaturate Monitor Colors
  By" and the accepted "Blend RGB Colors Using Gamma" values, and the exact list
  of built-in working-space presets, are not sourced here. Resolve from the CS6
  Help PDF (Color Settings section) or a CS6 dialog capture.
- **"Monitor RGB" working-space semantics.** Whether selecting it turns the RGB
  policy Off exactly (as a community source claims) needs confirmation. Resolve
  from the CS6 Help PDF or a controlled CS6 experiment.
- **Built-in profile provisioning.** Whether Kooka Pictura bundles sRGB, Adobe
  RGB, ProPhoto, and Gray profiles (licensing) or synthesizes them at runtime.
  Resolve with `00-overview/licensing-and-provenance.md`.
- **Adobe ACE parity target.** ACE's perceptual/saturation gamut mapping and BPC
  numerics are closed. Decide the numeric tolerance for "behavioral parity" and
  whether perceptual parity is a non-goal. Resolve in
  `11-cross-cutting/testing-strategy.md`.
- **Linux monitor profile source.** Whether to read `colord` over D-Bus, the
  Wayland color-management protocol, or both, and how to behave when neither is
  present. Resolve with a platform spike.
- **PSD profile round-trip fidelity.** Whether Photoshop rewrites the embedded
  ICC blob on save or preserves it byte-for-byte is not verified. Resolve by
  inspecting CS6-made PSDs.
- **32-bit limited adjustments.** The exact set of color commands available in
  32-bit mode is not sourced. Resolve from the CS6 Help PDF.
