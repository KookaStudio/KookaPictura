# Design

## Context

See `proposal.md` — Why. The engine's working pixels are 8-bit RGB or Grayscale. A CMYK PSD is normalized to RGB on read and keeps its CMYK planes in `source_planes` / `source_channels` so an unchanged save can re-emit them. The current CMYK path is a profile-free integer formula; RGB ICC profile handling already exists (`pictura-color`, `pictura-codec/src/icc.rs`, `icc_convert.rs`), and the app already has a `document_icc` display transform, a mode-preview mechanism, a Qt-boundary image decode/encode edge, and a session preference store. The bundled CC0 `assets/icc/photocraft-coated-cmyk.icc` is already used by the PDF writer.

## Goals / Non-Goals

**Goals:**

- One profile-aware CMYK path for PSD read, PSD write, `Image > Mode`, Convert to Profile, and the picker.
- One `ConversionOptions` model threaded through every conversion.
- Warnings, Embed Color Profile, and raster ICC handled at the app boundary without duplicating codec logic.
- Keep every touched file under the size gate by splitting along existing seams.

**Non-Goals:**

- A configurable RGB working space. The app's tool colours are 8-bit sRGB `QColor` values; a real RGB working space would change what those values mean and how every swatch displays. That is a separate change.
- `.csf`, regional presets, installed-profile discovery, Gray/Spot working spaces, Color Libraries, 16/32-bit picker fields, `Ask When Pasting`, and CMYK-to-CMYK Convert to Profile.
- Profile-aware Selective Color. Adobe documents it as CMYK sliders on RGB images with no working-space statement; libpsd's fixed integer path is the de facto reference. It stays with a `ponytail:` note.
- Native 16/32-bit CMYK ICC conversion. The working model is 8-bit; retained native CMYK planes are re-emitted unchanged, and an edit re-encodes at 8 bits and widens. This ceiling is existing and stays documented.

## Decisions

### CMYK profile priority and policy

The source of truth is, in order:

1. Resource `1039` when it parses as a CMYK profile.
2. The app's working CMYK profile (default `Profile::coated_cmyk()`).
3. The documented profile-free formula.

`OpenOptions` carries the policy, an optional working RGB profile (sRGB), an optional working CMYK profile, and `ConversionOptions`. `read_psd_with` remains the library convenience: an embedded CMYK profile is used, an untagged CMYK file takes the profile-free fallback. The app always supplies the working CMYK profile, so an untagged CMYK file is decoded through the working space instead of the formula.

Policy semantics for CMYK:

- **Preserve** decodes through the embedded (or working) profile, keeps `1039`, and leaves the retained CMYK planes alone. The working RGB is sRGB, so `document_icc` stays `None`.
- **Convert** re-encodes the retained CMYK planes through a CMYK→CMYK transform into the working CMYK profile, replaces `1039` with the working profile, and records the source bytes in `source_icc`. This is the closest analogue of Photoshop's Convert to Working CMYK in a working-RGB engine.
- **Off** decodes with the formula, drops `1039`, and records nothing.

*Alternative considered:* keep CMYK Preserve and Convert visually identical and only drop the tag. Rejected because it would make the policy a no-op and leave a misleading tag.

### Convert to Profile

The codec operation takes `ConversionOptions` and a `flatten` flag. Intent and BPC go into `Transform::new_flags`; dither is a deterministic ordered step applied to 8-bit output after the transform. The 16/32-bit native path keeps using `convert_native_document` but takes the same options; dither is ignored there.

Dither is intentionally not Adobe's closed kernel. A fixed 8x8 ordered matrix is deterministic, testable, and only masks banding; the design records it as behavioural approximation. The alternative, error diffusion, is slower and harder to compare across threads.

Flatten reuses `pictura_render::document_ops::layer_ops::merge::flatten` before the colour transform. Preview reuses the existing `mode_preview` mechanism: clone the pre-preview document, convert the clone, install it as the session document, and recomposite. Cancel restores the clone; commit discards it and converts the real document once.

### Raster ICC

Qt returns decoded RGB values; it does not apply ICC on this path. The decode edge therefore treats the decoded values as encoded in the embedded profile:

- JPEG: parse `APP2` `ICC_PROFILE` chunks and concatenate them.
- TIFF: read tag `34675`.
- PNG: read the `iCCP` chunk and inflate it.

A small codec probe exposes the profile bytes and the format; the app applies the policy. `Preserve` sets `document_icc` (the display path already converts to sRGB), `Convert` uses `pictura-color::convert`, and `Off` leaves the raw values untagged. `Place` always converts to the host document's sRGB working space before adding the layer.

For Save As JPEG/TIFF, the current raster path normalizes to sRGB. That is correct for Export As/Quick Export but wrong for Save As of a wide-gamut document. The new Save As path keeps the working RGB samples and either appends the profile (JPEG APP2, TIFF tag 34675) or writes untagged. Export As keeps the sRGB path. A CMYK source document's raster save writes the sRGB working pixels and does not mis-tag them with the CMYK profile.

*Alternative considered:* reuse the Export As encoder for Save As and embed sRGB only. Rejected because it would change the pixels of an Adobe RGB document on save, which Photoshop does not do.

### Warnings and embedding

The open pipeline probes the file before the decode worker commits. PSD/PSB use the existing image-resource parser; raster formats use the new ICC probe. If the matching ask preference is on and the profile is mismatched or missing, the app shows the warning and maps the answer to a per-open policy override:

- Use embedded / Assign working space → Preserve
- Convert to working space / Assign then convert → Convert
- Discard / Leave without a profile → Off

The override is passed to `read_psd_with_options` or the raster decode edge; it is not persisted. Cancelling the warning aborts the open. Warnings are app-global preferences and never create history.

Embed Color Profile is a Save As option for PSD/PSB/JPEG/TIFF. For PSD/PSB it controls resource `1039` in the cloned save document; for JPEG/TIFF it controls the APP2/tag 34675 segment. The toggle never changes pixels. PNG/WebP/BMP do not offer it, matching Photoshop's PNG behaviour. PDF keeps its own profile-inclusion policy.

### Picker

The picker's working-CMYK projection and gamut test live in a new Rust bridge module (free functions over a global app `ColorSettings`), so the C++ picker does not link lcms2 or duplicate profile parsing. The service exposes:

- `picker_cmyk(r,g,b) -> [c,m,y,k]`
- `picker_rgb(c,m,y,k) -> 0xRRGGBB`
- `picker_gamut(r,g,b) -> (out_of_gamut, closest_rgb)`

The authoritative picker value stays RGB/HSV. CMYK edits convert through the working profile; repeated edits can drift by a rounding step, which is acceptable and documented. Gamut is `rgb → cmyk → rgb'`; the alert threshold is a per-channel difference greater than 1 LSB, to avoid false positives from the round trip. The picker preference is a session value; `ColorPickerDialog::getColor` branches to `QColorDialog` for `System`, while direct test constructions stay on the Adobe widget.

*Alternative considered:* compute CMYK in C++ with `QColor::toCmyk`. Rejected because it is profile-free and would make the picker disagree with the document conversion.

### File layout and size

New bridge functions go in `cxxqt_object/color_management.rs`; new dialogs are `profile_warning_dialog.{h,cpp}` and `save_options_dialog.{h,cpp}`; the picker is split into `color_picker_widget.{h,cpp}` (the `ColorPicker`, plane, ramp, compare) and `color_picker_dialog.{h,cpp}` (the host and Add to Swatches). All new `.cpp` files are added to `CMakeLists.txt` explicitly. This keeps `cxxqt_object.rs`, `impl_core.rs`, `frame.cpp`, and `color_picker_dialog.cpp` under the 1200-line cap.

## Risks / Trade-offs

- **Synthetic CMYK profile** → the bundled profile is a modelled press, not SWOP. Parity claims are against lcms2, and the spec/roadmap state the deviation. A user-supplied ICC can be added later without changing the transform path.
- **CMYK→CMYK Convert can shift colours** → that is the intent of Convert; Preserve keeps the numbers. The per-open warning lets the user choose.
- **Qt JPEG/TIFF ICC support** → the writers may not preserve arbitrary ICC metadata through `QImageWriter`. The design writes the profile into the container after the encode (APP2/tag injection) rather than relying on Qt, and tests parse the output independently.
- **Dither is not Adobe's kernel** → behavioural approximation, deterministic tests, 8-bit only.
- **Warning probe false negatives** → a malformed or unusual container may skip the warning. The decode still succeeds; the profile is ignored, which is the safe fallback.
- **File splits** → pure moves; tests and `CMakeLists.txt` entries must be updated in the same change.
- **Existing CMYK goldens change** when a profile is present. The change updates the oracle and adds a separate profile-free fixture so the fallback stays pinned.

## Migration Plan

- No document or preference migration. Existing sessions without the new preference keys use the documented defaults (CMYK policy Preserve, working CMYK coated, relative intent, BPC on, dither on, picker Adobe, warnings off).
- Existing profile-free CMYK fixtures and tests remain as fallback tests; new profile-bearing fixtures cover the lcms2 path.
- The save path change for JPEG/TIFF is visible only when a document has a non-sRGB RGB working profile; sRGB documents keep their current bytes. Rollback is removing the Save As option and restoring the sRGB normalization.
- Archive the change with the PR; update the parity register rows for #568/#569/#600/#604/#809 and add the raster ICC rows.

## Open Questions

None that change the specs or the task breakdown. The deferred items are explicit non-goals.
