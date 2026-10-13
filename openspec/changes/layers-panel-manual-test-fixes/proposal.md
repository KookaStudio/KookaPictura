# Proposal

## Why

A manual-test pass over the Layers panel (`PAN-001`) turned up six small,
independent visual/interaction defects. Each is self-contained, the underlying
models already exist, and they share one surface, so they close together as the
same shape of pass as the earlier `layers-panel-surface-completeness` and
`crop-straighten` UX passes.

## What Changes

- **Opacity label alignment** — the Opacity field's leading label shares its
  slack with the value box; it SHALL be right-aligned against that box instead of
  floating at the row's left edge.
- **Mask add activates the mask** — adding a layer mask currently leaves the
  pixel thumbnail active. A freshly added raster mask SHALL become the view's
  mask edit target, and the panel's active-thumbnail brackets SHALL follow it, so
  painting/filling edits the mask at once (CS6 behaviour).
- **Group expansion survives a structural change** — adding, removing, or moving
  a layer shifts a group's index-based path, which collapses the group. The panel
  SHALL re-find the group by its ancestor name chain and keep its expanded state
  (this also covers dropping a layer into an open group).
- **No outline on a group icon** — the 1 px black thumbnail outline is currently
  drawn around every row's thumbnail box, including a group's folder glyph; a
  group SHALL draw no outline, and its folder glyph SHALL grow from half to about
  three quarters of the thumbnail.
- **Thumbnails fill the row vertically** — the thumbnail box SHALL span the row's
  full height (minus a small pad) so a tall document's thumbnail fills the row
  rather than floating in a square box.
- **Chain glyph size and rotation** — the mask/vector link glyph SHALL be drawn
  two pixels larger than its slot and rotated a quarter turn so it reads
  vertically between the side-by-side thumbnails, in both linked and unlinked
  states.
- **Combo affordances** — the blend-mode and filter combo boxes SHALL draw a
  drop-down caret (a small down-pointing triangle), and Up/Down SHALL cycle the
  options while the field is focused, without the value change dropping focus.
- **Blend-change history label** — a blend-mode edit SHALL be recorded as
  `Blending Change: <mode name>` (e.g. `Blending Change: Color Burn`) instead of
  the generic `Blend Mode`.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the Opacity label is right-aligned; the delegate draws no
  outline on a group icon (and draws it larger), fills the thumbnail box to the
  row height, and draws a larger, rotated chain glyph; a group's expanded state
  survives a structural change; the combos show a drop-down caret and cycle with
  Up/Down without losing focus; a blend-mode edit is labelled
  `Blending Change: <mode name>`.
- `compositing/layer-masks`: adding a raster mask makes it the edit target.

## Impact

- Assets: two combo-caret SVGs under `assets/ui/` (qrc + PROVENANCE).
- Rust: `BlendMode::name` (the CS6 display name) in `pictura-core`; the blend
  bridge labels its undo state `Blending Change: <name>`; `layer_mask_add_path`
  sets the mask edit target on success.
- App: `theme.cpp` draws the combo caret via the stylesheet; `panels/layers_panel.cpp`
  right-aligns the Opacity label, derives the active thumbnail from the mask
  target, remaps expansion by name chain, drops the raster target when a vector
  mask is clicked, and keeps a focused header control's focus across a refresh;
  `panels/layers_panel_model.h` gains `namePathForPath`/`indexForNamePath`;
  `panels/numeric_field.{h,cpp}` gains a label-alignment setter;
  `panels/layers_panel_internal.h` gains the full-height thumbnail slot, the
  fixed letterbox math, the group outline skip, the larger folder glyph, and the
  rotated/larger chain glyph.
- Tests: Qt Test cases in `cpp/tests/tst_layers_panel_fixes.cpp` (one per fix)
  and an updated `cpp/tests/tst_mask_edit.cpp`; a `pictura-core` unit test for
  `BlendMode::name`.
- No new dependencies. No `docs/` changes.
