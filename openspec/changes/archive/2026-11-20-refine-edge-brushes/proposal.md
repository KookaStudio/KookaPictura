# Proposal

## Why

Refine Edge ships without its interactive refinement brushes. The archived
`refine-edge` change explicitly deferred them (issue #612, parity row SEL-4),
so a soft edge such as hair or fur can only be refined by the global sliders,
not by brushing where the edge estimate should apply. CS6 exposes Refine
Radius / Erase Refinements, and Photoshop CC 2015.5+ carries the same job in
the Select and Mask workspace as the Refine Edge Brush with an Alt restore
modifier. This change closes the brush-interaction gap on the existing canvas;
the estimator's strand-recovery ceiling is recorded rather than claimed.

## What Changes

- Add ordered brush dabs to the Refine Edge engine: a refine dab widens the
  edge-estimation band locally, a restore dab reverts the input coverage in
  its disk, and later dabs win. Strokes are recorded in document pixels; the
  preview scales their coordinates and radii by the proxy's linear factor
  before refining, and OK passes them unchanged. The engine applies each dab
  in the pixel space of the coverage it is given.
- Add the refinement-band output (`edge_band_from`) behind the dialog's Show
  Edge view.
- Add the brush to `RefineEdgeDialog`: size (a diameter in document pixels,
  default 30 px, 1–1000), refine and restore modes, Alt restores only while
  held, `Shift+E` as the latched CS6 alias, bracket sizing, and `J` / Show
  Edge (the CS6 Show Radius view).
- Arm the main canvas while the dialog runs: brush ring on hover, left-drag
  strokes, and a transient on-canvas preview of the refined contour plus the
  refinement band. The preview is a view concern; the document changes only
  on OK, and Cancel remains byte-identical.
- Pass the strokes through the Rust bridge to both the menu's Refine Edge and
  the Layers panel's Refine Mask, so both entry points brush.
- Record the CS6 and CC generations in the Refine Edge documentation, mark
  parity row SEL-4 SHIPPED with the geometry/approximation ceiling recorded,
  and tick the ROADMAP item.

Explicitly out of scope (recorded as ceilings): the full CC Select and Mask
workspace, Onion Skin, the mask-painting Brush tool, Refine Hair, Color/Object
Aware refine modes, High Quality Preview, Remember Settings/presets,
canvas view-mode rendering, and the estimator work needed for the docs'
strand-recovery parity criterion.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/refine-edge`: the dialog requirement gains the refinement brush,
  brush size, Alt/`Shift+E` restore, bracket sizing, and Show Edge; a new
  requirement defines the brush-dab band override, restore ordering, proxy
  scaling, and the refinement-band output.

## Impact

- `crates/pictura-select/src/refine.rs` and `lib.rs`: `RefineDab`, brush
  field, `refine_from` dabs parameter; `lib.rs` exports `RefineDab` and
  `edge_band_from`.
- `crates/pictura-app/src/cxxqt_object/impl_selection/refine.rs` and
  `refine/imp.rs`: dab wire format, band previews, preview scaling, apply.
- `crates/pictura-app/cpp`: `refine_edge_dialog.{h,cpp}`,
  `menu_select.cpp`, `panels/layers_panel_mask_menu.cpp`, `tools.{h,cpp}`,
  `tools_marquee.cpp`, `dialogs.{h,cpp}`, `menu_edit.cpp`,
  `image_view.{h,cpp}`, `image_view_document.cpp`, and
  `tests/tst_refine_edge.cpp`.
- `docs/08-selection/refine-edge.md`, `docs/dev/STATE.md` (ceiling paragraph,
  currently lines 5220-5223), `docs/dev/doc-code-parity-gaps.md`, `ROADMAP.md`.
- No new dependencies; no new C++ translation unit; no new capability.
