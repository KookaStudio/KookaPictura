# Proposal

## Why

CMYK conversions still use a profile-free integer formula, so a round trip through CMYK drifts even when the document carries an ICC profile (#809). Around that gap, the colour-management UI is a stub: Color Settings shows a fixed sRGB label and one policy combo (#569), Convert to Profile is a bare destination combo with a hard-fixed intent (#568), and the colour picker has no web/gamut alerts, no editable CMYK, no picker preference, and disabled libraries (#600, #604). Raster imports ignore embedded ICC profiles and raster saves always normalise to sRGB, so colour management stops at the PSD boundary.

This bundle closes the engine gap first, then builds the settings, conversion, warning, embedding, and picker surfaces on it. It uses the existing CC0 `Photocraft Coated CMYK` profile and the existing lcms2 path, so no new dependency or Adobe profile is introduced.

## What Changes

- **CMYK through ICC (#809).** CMYK read, write, and `Image > Mode` conversion use the document's embedded CMYK profile, else the configured working CMYK profile; the profile-free formula remains the documented fallback when no profile is available. `Image > Mode > CMYK` attaches the working CMYK profile so a save round-trips.
- **Color Settings (#569).** The dialog gains a working CMYK space, per-mode RGB and CMYK incoming policies, conversion options (engine, intent, black-point compensation, dither), and mismatch/missing ask toggles. The RGB working space stays fixed sRGB; `.csf`, Gray/Spot spaces, installed-profile discovery, and regional presets are deferred.
- **Convert to Profile (#568).** Destination, intent, BPC, dither, flatten, and an on-canvas preview. The conversion is one undo state and keeps the 16/32-bit native samples on the existing lcms2 path.
- **Profile warnings.** When enabled, opening a PSD/PSB or a raster image with a mismatched or missing embedded profile asks before the document is committed. Warnings are off by default and never create history.
- **Embed Color Profile.** Save As offers the per-save toggle for PSD/PSB/JPEG/TIFF; PSD/PSB use resource 1039, JPEG uses APP2, TIFF uses tag 34675. PNG/WebP/BMP do not offer it, matching Photoshop's PNG behaviour. PDF keeps its own profile-inclusion policy.
- **Raster ICC.** JPEG, TIFF, and PNG imports parse the embedded ICC profile and apply the incoming policy (Preserve/Convert/Off) instead of silently treating every raster as sRGB. JPEG and TIFF saves can preserve the document's working profile and embed it.
- **Colour picker (#600, #604).** A web-safe alert cube, an out-of-gamut triangle with the closest working-CMYK swatch, editable CMYK fields through the working CMYK profile, and a Preferences > General picker choice (Adobe / System). Color Libraries and 16/32-bit picker fields are deferred.
- **Ceilings kept explicit.** RGB working space remains sRGB; Selective Color stays libpsd-compatible; `.csf`, Color Libraries, HDR/32-bit picker fields, Gray/Spot spaces, and `Ask When Pasting` are out of scope.

## Capabilities

### New Capabilities

- `color/profile-warnings`: the mismatch and missing-profile decision flow and its ask preferences, shared by PSD/PSB and raster imports.
- `interop/icc-embedding`: the per-save `Embed Color Profile` option, its default state, and the format support matrix (PSD/PSB/JPEG/TIFF versus PNG/WebP/BMP/PDF).

### Modified Capabilities

- `color/color-management`: adds the bundled CMYK working profile, profile-aware RGB↔CMYK conversion, and the shared conversion options.
- `color/color-settings`: adds the working CMYK space, CMYK policy, conversion options, and warning toggles to the persisted settings and dialog.
- `color/color-profile-assignment`: Convert to Profile accepts conversion options, flatten, and preview; the destination and source profile rules are made explicit.
- `color/image-mode-conversion`: CMYK mode conversion becomes profile-aware and attaches the working CMYK profile.
- `codec/psd-color-modes`: CMYK read/write uses the embedded/working profile with a profile-free fallback; the profile-bearing fixture and oracle are added.
- `codec/psd-icc-conversion`: the ICC pass covers CMYK as well as RGB and records the source profile for the CMYK policy.
- `interop/image-import`: JPEG/TIFF/PNG decode exposes the embedded ICC profile and applies the incoming policy before the document is built.
- `interop/raster-export`: Save As for JPEG/TIFF preserves the document working profile and embeds or omits the ICC profile; Export As remains sRGB-normalised.
- `ui/color-picker`: web and gamut alerts, editable CMYK fields, and the picker preference are added to the picker contract.

## Impact

- **Engine:** `pictura-color` (CMYK profile/options/dither), `pictura-codec` (CMYK read/write, PSD ICC resource, raster ICC probe), `pictura-render` (profile-aware mode conversion), `pictura-adjust` unchanged.
- **App:** `PictureView` colour settings, the Color Settings and Convert to Profile dialogs, new warning and save-option dialogs, the open pipeline (PSD and raster), the raster save/import edges, the picker, and Preferences > General. New bridge functions go in new modules so `cxxqt_object.rs`, `impl_core.rs`, and `frame.cpp` stay under their size caps.
- **Tests:** new Rust lcms2/oracle coverage for CMYK and raster ICC; Qt Test suites for Color Settings, Convert to Profile, warnings, save options, and picker alerts; updates to the existing CMYK color-mode and ICC suites and their golden expectations.
- **Specs:** the profile-free CMYK requirements in `codec/psd-color-modes` and the fixed-sRGB assumptions in `color/color-settings` are intentionally replaced; the old scenarios are updated rather than deleted silently.
- **Dependencies/assets:** no new crate dependency. The existing CC0 `assets/icc/photocraft-coated-cmyk.icc` becomes the default working CMYK profile; its provenance row stays authoritative.
- **Deferred:** `.csf` and regional presets, Color Libraries/PANTONE, 16/32-bit picker fields, configurable RGB working space, Gray/Spot spaces, Selective Color profile-awareness, `Ask When Pasting`, and CMYK-profile-to-CMYK-profile Convert to Profile.
