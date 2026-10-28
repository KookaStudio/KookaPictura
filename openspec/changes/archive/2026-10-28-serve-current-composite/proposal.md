# Proposal

## Why

About two dozen app call sites that only need "the picture as currently shown" —
Magic Wand, Quick Selection, Grow, Similar, Sample All Layers, Magic Eraser,
Paint Bucket with All Layers, the Channels panel rows, Copy Merged,
Replace/Match Color, Color Range, Refine Edge, Magnetic Lasso, Duplicate Merged,
Trim, and the raster exports — call `helpers_composite::current_buffer`, which
composites the whole layer stack through `pictura_render::composite_active` on
every call. The document already maintains `doc.composite` as the current
picture: the edit transaction patches it regionally or stores it whole after
every pixel-changing edit and preview. So a wand click on a 4000²×8-layer
document pays a full multi-layer `f32` composite for a frame the document is
already holding.

The root cause is a seam leak: "the picture as currently shown" is not a module,
so each caller reaches the raw compositor and re-derives a frame that the
document session already owns.

## What Changes

- Add one accessor for "the picture as currently shown" that serves the
  document's maintained composite when it is fresh and composites the stack only
  when it is not, borrowing the cached frame instead of copying it.
- Move the read-only call sites to that accessor: selection sampling (wand,
  Quick Selection, Grow, Similar, Magnetic Lasso, Color Range, Refine Edge, the
  Channels panel rows), paint sampling (Paint Bucket with All Layers, Magic
  Eraser, Sample All Layers for Smudge/Blur/Sharpen/Mixer), clipboard (Copy
  Merged), image adjustments (Duplicate Merged, Trim, Replace Color, Match
  Color), and the raster exports (Export As, Save for Web, and the Save As
  format encoder).
- Keep `current_buffer` for the sites that genuinely need a composite: derived
  documents (transform previews, a hidden-layer PDF background), open-file
  normalization, the repair paths that feed `store_composite`, and the main Save
  and PDF save paths, which refresh the retained native composite first and so
  would otherwise encode the native narrowing rather than the composite they
  encode today.
- State the freshness contract the accessor relies on, so a future edit path
  that leaves the composite stale is a spec violation rather than a silent
  regression.

No behavior change is intended: the accessor serves the composite only where it
is provably equal to a fresh composite, and composites otherwise.

## Capabilities

### New Capabilities

- `compositing/current-picture`: the app answers "the picture as currently
  shown" from the document's maintained composite when it is fresh, and
  composites the layer stack only when it is not.

### Modified Capabilities

None.

## Impact

- New module `crates/pictura-app/src/cxxqt_object/picture.rs` (the accessor and
  its tests); `PictureViewRust` gains the "is a stroke presenting into the layer
  pixels" query that the QObject already exposes.
- ~22 call sites in `crates/pictura-app/src/cxxqt_object/` move to the accessor:
  `clipboard`, `color_range`, `export`, `image_adjust/image_ops`,
  `image_match_color`, `image_replace_color`,
  `impl_selection{,/quick,/refine}`, `magnetic`, `paint_tools{,/fills}`,
  `save_format`, `web_export`.
- `impl_core/save.rs` and `pdf_export.rs` keep `current_buffer` because they
  refresh the retained native composite before reading the picture.
- `current_buffer` and `composite_active` are unchanged; the compositor's
  contract and the edit transaction's freshness guarantees are unchanged.
- No new dependencies. No PSD/codec/GPU surface changes.
