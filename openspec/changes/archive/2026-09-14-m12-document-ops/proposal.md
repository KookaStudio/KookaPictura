## Why

M10 shipped the destructive `Image > Image Size / Canvas Size / Image Rotation`
math as pure functions over a single `PixelBuffer` in `pictura-ops`, but those
functions know nothing about a document: they cannot resample a layer tree, move
layer and mask bounds, remap groups and masks, or refresh the cached composite.
The document-level behavior described by `docs/04-image-ops/image-size.md`
(`IMG-001`), `canvas-size.md` (`IMG-002`), and
`image-rotation-and-flip.md` (`IMG-003`) has no OpenSpec contract yet, so the
recursive layer/group/mask handling and the psd-tools structural oracle have no
reviewable target.

## What Changes

- Add a **`document-resize`** capability: `pictura_render::resize_document(doc,
  width, height, resample)` resamples every pixel layer's color and alpha
  channels, every decoded layer/group mask, and every document-level channel
  through the M10 `pictura_ops::resize` kernel, scales each layer and mask
  `rect`, sets the document size, and recomputes the composite. It recurses
  through groups; adjustment layers carry no color channels.
- Add a **`document-canvas`** capability:
  `pictura_render::resize_canvas_document(doc, width, height, anchor)`
  translates layer and mask `rect`s by the nine-anchor offset, re-extends or
  crops the document-level channels with a zero fill, sets the document size,
  and recomputes the composite. Added canvas is transparent.
- Add a **`document-orientation`** capability:
  `pictura_render::rotate_document(doc, quarter_turns)` (1 = CW, 2 = 180,
  3 = CCW; any other value is `OpsError::InvalidParams`) and
  `pictura_render::flip_document(doc, horizontal)` apply the exact M10 index
  remaps to every layer channel, mask, and document-level channel and to every
  layer/mask `rect`; 90° and 270° swap the document dimensions; the composite is
  recomputed; four quarter turns and a doubled flip are identities.
- All three ops validate up front: `width`/`height` below 1 (and an out-of-range
  `quarter_turns`) return `OpsError::InvalidParams` with the document left
  bit-identical, and a malformed pixel-layer channel or mask is rejected before
  any mutation.
- `pictura-render` gains a normal dependency on `pictura-ops` (reusing
  `resize`, `rotate90_cw`/`rotate90_ccw`, `rotate180`,
  `flip_horizontal`/`flip_vertical`, `Resample`, `Anchor`, and `OpsError`) and
  dev-dependencies on `pictura-codec` and `pictura-testkit` for the oracle.
- Oracle: a `crates/pictura-render/tests/document/document_oracle.rs` that writes the
  transformed document with `pictura_codec::write_psd`, re-reads it for the
  structural round-trip, opens it with the independent `psd-tools` library to
  confirm document dimensions and layer count, and asserts `doc.composite ==
  composite_rgba(&doc)`. It skips cleanly when `psd-tools` is absent and is not
  marked `#[ignore]`.
- Out of scope (later): arbitrary-angle document rotation, the Bicubic Smoother
  / Bicubic Sharper / Bicubic Automatic resample methods, CMYK / Lab /
  Multichannel and 16-/32-bit documents, the app `Image` menu UI and dialogs,
  undo/history recording, and Smart Object source-pixel semantics.

## Capabilities

### New Capabilities

- `document-resize`: resample a whole document's pixel layers, masks, groups,
  and document-level channels, scale layer/mask bounds, set the document size,
  and recompute the composite.
- `document-canvas`: grow or crop the document canvas, translate layer and mask
  bounds by the nine-anchor offset, re-extend the document channels, and
  recompute the composite.
- `document-orientation`: rotate the document by exact quarter/half turns and
  flip it horizontally or vertically, remapping every channel, mask, and bound,
  and recompute the composite.

### Modified Capabilities

## Impact

- `crates/pictura-render/src/document_ops/` (new): `resize_document`,
  `resize_canvas_document`, `rotate_document`, `flip_document`, and the shared
  layer-tree traversal/rect-remap helpers.
- `crates/pictura-render/src/lib.rs`: re-export the four document operations.
- `crates/pictura-render/Cargo.toml`: add `pictura-ops`; add `pictura-codec`
  and keep `pictura-testkit` as dev-dependencies for the oracle.
- `crates/pictura-render/tests/document/document_oracle.rs` (new): the psd-tools
  structural round-trip and the `composite == composite_rgba` check; reuses
  `scripts/validate_output.py` when present.
- Dependencies: one new normal dependency (`pictura-ops`) and one new
  dev-dependency (`pictura-codec`); no new external crates.
- Follows `docs/04-image-ops/image-size.md` (`IMG-001`), `canvas-size.md`
  (`IMG-002`), and `image-rotation-and-flip.md` (`IMG-003`); the M10
  `image-resize`, `canvas-operations`, and `image-orientation` buffer-level
  specs and every file under `docs/` are not modified.
