# Design

## Context

See `proposal.md` — Why. The app keeps the working model 8-bit and retains a 32-bit source store (`Document.source_planes`) for save and native compositing. `level0_buffer` builds the canvas frame from `doc.composite`; the Info panel samples `sample_argb` from the same composite; the ordinary picker and `ColorState` are `QColor`-based. `crates/pictura-app/src/cxxqt_object.rs` is exactly 1200 lines (the code cap), and `color_picker_dialog.cpp` is 1181 lines.

## Goals / Non-Goals

**Goals:**

- A display-only preview that can show HDR values above white without touching stored pixels.
- True 32-bit Info readouts from the current source, including samplers.
- An HDR picker that can author and round-trip float colours, including Add to Swatches and session persistence.
- Keep the 8-bit picker and all existing 8/16-bit behaviour unchanged.

**Non-Goals:**

- Persisting the View preview white point in PSD/PSB/TIFF. CS6 stores it; this change keeps the preview per-view and records the gap.
- 32-bit painting or tool colour consumption. Tools keep the clamped `QColor`; the HDR picker authors the colour state.
- A true float colour model for CMYK/Lab readouts; those stay derived from the 8-bit sample.
- Changing the Histogram's 32-bit binning.

## Decisions

### D1: Apply the preview when building level 0, not to the document composite

`reset_pyramid` rebuilds `level0` from `level0_buffer(doc)`. The preview becomes a view-level parameter to that builder: for a 32-bit document it reads the current 32-bit source, applies the display transform, narrows to 8 bits, and passes the result through the existing `buffer_to_srgb(doc, …)` display transform before the frame is cached. `doc.composite` remains the stored/composited truth used by tools, save, and history.

Alternative considered: overwrite `doc.composite` on preview change. Rejected because a display command would then alter painting, save, and undo state.

### D2: Current source, not a stale open-time snapshot

The retained store is refreshed only on save, PDF export, or recovery. Ordinary edits update the composite/layers. The preview and the Info sampler therefore use the retained `f32` store only while it is current and the current composite after an edit. The app carries a `native_store_stale` signal set by document edits and cleared by `refresh_native_composite`; the conversion change uses the same signal. This prevents a preview rebuild from snapping back to pre-edit pixels.

### D3: Status-bar exposure and dialog share one per-view state

`HdrPreviewOptions { method, exposure_ev, gamma }` lives on `PictureViewRust`, defaults to Exposure & Gamma with `0`/`1.0`, and resets when a document is opened. The dialog and the new status-bar `32-Bit Exposure` slider both edit it; the slider double-click resets the exposure to `0`. The preview never enters a history snapshot and is not serialized.

### D4: Unclamped f32 accessor, separate from the compositor's unit helpers

`native_unit`/`emit_native` clamp to `[0,1]`, so they cannot feed a 32-bit readout that must show values above white. A new free-function bridge samples the retained `F32` planes directly (or the current composite when stale) and returns the raw values. The Info panel uses it when its block depth is 32; the existing `sample_argb` path remains the fallback. Samplers pass their size and scope through the same bridge.

### D5: Separate HDR picker class

`color_picker_dialog.cpp` is 1181 LOC, so the HDR picker is a new `HdrColorPickerDialog` with its own files. It shares the existing colour field/ramp/comparison widgets but adds the Intensity slider, float fields, preview swatches, and the HDR `Add to Swatches` path.

The float value is the picker's sRGB colour converted to linear light and multiplied by `2^intensity_stops`; the `QColor` mirror is the clamped sRGB encoding. Non-finite input clamps at the finite boundary; unparsable input leaves the colour unchanged. The Intensity range is `-10..+10` stops in `0.1` steps (inferred; the docs give no bounds). Preview Stop Size defaults to `3` and Relative to Document to off.

### D6: Entry points route through the active document

`ForegroundBackgroundWidget` and the Color panel currently know only `ColorState`. The picker entry points move behind a small opener that asks the main window for `activeView()->document_depth_bits()`, so a 32-bit document opens `HdrColorPickerDialog` and an 8/16-bit document opens the ordinary dialog. Clicking an active Color panel swatch opens the picker; clicking an inactive one selects it first. The other `ColorPickerDialog::getColor` call sites (options bars, fill/stroke/type dialogs) remain ordinary 8-bit pickers in this change and are recorded as a ceiling.

### D7: Swatch storage and persistence

`Swatch` gains an optional `HdrColor` triple. `SwatchGrid` gains an HDR-aware selection signal (or a `Swatch`-returning signal) so a click restores both the float state and the `QColor` mirror. `frame_session.cpp`/`frame_build.cpp` serialize and restore the optional triple; an 8-bit swatch keeps the existing `{name, color}` shape. The session-store test covers the new field and the backward-compatible default.

### D8: Free-function bridges and CMake

`cxxqt_object.rs` is at the cap, so the preview state accessors, the float sampler, and the preview-changed signal live in a free-function bridge module, following `image_hdr_toning.rs`/`image_adjust/new_document.rs`. New dialogs and bridge files are added explicitly to `CMakeLists.txt`; the new Qt suite is added to the tests CMake list.

### D9: File-persisted preview is a ceiling

CS6 stores the View preview settings in PSD/PSB/TIFF and applies them on reopen. This change does not write image resource 1070 or an equivalent; the spec and parity register record the omission and a follow-up issue tracks it. The per-view status-bar exposure is implemented because it is the display path the app can support without touching the file.

## Risks / Trade-offs

- **Preview and composite diverge** → Deliberate: the preview is display-only. Tests assert the composite, history, and save inputs are unchanged.
- **A displayed value above `1.0` is lost after an edit on an existing 32-bit document** → Existing `emit_native` ceiling; the current-source rule at least prevents showing pre-edit pixels.
- **The HDR picker can author a colour that 8-bit tools clamp** → `ponytail:` ceiling documented in the picker and proposal; the float value is preserved in `ColorState`/swatches for future 32-bit tools.
- **Preview changes rebuild the full level-0 frame** → Acceptable for an explicit display command; no per-cursor preview.
- **The new swatch field changes internal storage** → Keep the existing integer constructor and serialization default so 8-bit swatches remain byte-identical; the float triple is optional.
- **The Info panel's CMYK/Lab/HSB modes still use the 8-bit sample** → Explicit ceiling; the new requirement only claims Actual Color/Proof Color/RGB/Grayscale 32-bit precision.
- **Other picker call sites still open the 8-bit dialog** → Explicit ceiling; the foreground/background and Color panel entry points are the routed surface.
- **`cxxqt_object.rs` at the cap** → New bridges are free functions; no new invokables in the root.
