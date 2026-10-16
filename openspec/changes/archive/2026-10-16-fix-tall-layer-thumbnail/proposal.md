# Proposal

## Why

A tall document's layer thumbnail is drawn squashed horizontally, with a strip
of transparency down the right edge. Two aspect assumptions disagree:

- The bridge's `layer_thumbnail_positioned` builds a **square** `size × size`
  image (`crates/pictura-app/src/cxxqt_object/helpers_composite.rs`), placing the
  layer at its document-relative offset scaled by `size / max(doc_w, doc_h)`.
  For a tall document a full-width layer occupies the full height but only
  `doc_w/doc_h` of the width, leaving transparent columns on the right.
- The delegate letterboxes the paint target to the **document aspect**
  (`layers_panel_internal.h` `letterboxedThumb`) and then stretches the whole
  source into it with `painter->drawImage(shaped, image)`, which ignores the
  source's own aspect.

So a square, right-padded source is squeezed into a narrow document-aspect rect.
The panel's `ui/layers-panel` capability already promises a document-aspect
thumbnail, but only the drawn rect honours it; the source image does not.
`shape_thumbnail` has the same square-over-document-extent shape and reaches the
same `drawImage` branch (shape rows report `kind == "pixel"`), so shape layers
squash for the same reason.

## What Changes

- Make the bridge emit a **document-aspect** thumbnail for Entire Document
  placement: `layer_thumbnail_positioned` SHALL size its output `tw × th` from
  the document aspect (the same `tw`/`th` rule `layer_thumbnail_image` uses) and
  place the layer's offset within it, so a full-width layer fills the full width
  with no transparent strip.
- Make `shape_thumbnail` emit the same document-aspect output over the document
  extent, so shape layers are not squashed either.
- The delegate keeps its letterboxed rect, so `thumbRect` and all
  hit-testing/adjacency math are unchanged; source aspect now equals target
  aspect, so `drawImage` no longer distorts.
- Add a Rust unit test for a tall-document positioned thumbnail and a Qt Test
  regression asserting a tall-doc full-width layer's thumbnail has document
  aspect with no transparent right column.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the "Layer thumbnail preserves the canvas aspect ratio"
  requirement is extended — the thumbnail **image** built for Entire Document
  placement (and the shape thumbnail) SHALL itself carry the document's aspect
  ratio, not only the rect it is drawn into, so a full-width layer in a
  non-square document has no transparent padding strip.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` —
  `layer_thumbnail_positioned` output aspect.
- `crates/pictura-app/src/cxxqt_object/shapes.rs` — `shape_thumbnail` output
  aspect.
- `crates/pictura-app/src/cxxqt_object/tests.rs` — tall-document unit test.
- `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` — Qt regression.
- No change to `thumbRect`/hit-testing, the delegate's letterbox math, the
  document, the codec, or any dependency.
