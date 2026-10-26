# Proposal: lens-correction-smart-filter

## Why

Issue #934 (follow-up to #347). Lens Correction (#950) commits through the
ordinary filter path, so on a Smart Object it filters the pixel proxy
destructively: the correction is baked and cannot be re-edited. CS6 instead
adds a Smart Filter that re-opens the dialog, keeping the settings in the
`filterFX` descriptor. The Smart Filter model decodes only the Camera Raw
filter (`pictura-render/src/smart_filter.rs`), so a Lens Correction smart
filter needs a descriptor codec, a render op, and the smart-object commit and
re-edit paths.

Per the #934 decision, this is **Pictura-only**: the descriptor's filter id,
name, and keys are our own (the reference Photoshop Lens Correction smart
filter's id and key map are not available), so no cross-app interop is
claimed. The Camera Raw smart filter (`filterID 2683`) stays the only
reference-compatible one.

## What Changes

- `pictura-codec` adds a `lens_correction` module: the smart filter's
  descriptor (our own `filterID`, class id, name, and short keys), typed
  encode/decode of the dialog's 16 slots, and
  `attach_lens_correction_filter`, all on the existing generic
  `attach_smart_filter` (so unmodeled `Fltr` keys and `blendOptions` survive a
  re-edit, and a foreign filter id is preserved untouched).
- `pictura-filters` moves the slots→`LensCorrectionParams` mapping out of the
  app bridge (`LensCorrectionParams::from_slots`, `SLOTS`) so the render crate
  can build the parameters from a stored record.
- `pictura-render` adds `SmartFilterOp::LensCorrection` to the smart-filter
  chain and a `document_ops::lens_correction` pair: `apply_lens_correction`
  (render the smart source, filter, bake the proxy, attach the smart filter)
  and `lens_correction_slots` (read the stored record back).
- `pictura-app` routes a `lens-correction` commit on an embedded Smart Object
  to the smart path (attaching a smart filter, one history state, Last Filter
  and Last Filter Settings recorded with the 16 slots); a plain raster layer
  keeps the destructive commit. The dialog session reads its preview from the
  smart object's source and prefills from the stored record, so a re-edit
  never re-applies the correction.

## Capabilities

### New Capabilities

- `imaging/lens-correction-smart-filter`: the descriptor codec, the
  smart-object apply, the composite render, and the dialog re-edit.

## Impact

- `pictura-codec` (`lens_correction/*`, `lib.rs` exports),
  `pictura-filters` (`distort/lens_correction/mod.rs`), `pictura-render`
  (`smart_filter.rs`, `document_ops/lens_correction.rs`, exports),
  `pictura-app` (`filter_tools/lens_correction.rs`, `filter_tools.rs`,
  `filter_tools/jobs.rs`, `lens_correction_dialog.cpp`, `build.rs`).
- `docs/dev/doc-code-parity-gaps.md` (FILT-10 closes), `ROADMAP.md`.
- No change to the `imaging/lens-correction` capability: its destructive
  requirements still hold for a raster layer.
