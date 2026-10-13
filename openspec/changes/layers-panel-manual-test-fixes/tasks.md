# Tasks

## 1. Opacity label alignment

- [x] Add `NumericField::setLabelAlignment`.
- [x] Right-align the Layers panel Opacity label.
- [x] Qt check: `tst_layers_panel_fixes::opacityLabelRightAligned`.

## 2. Mask add activates the mask

- [x] Set `mask_edit_target` on a successful `layer_mask_add_path`.
- [x] Derive the panel active thumbnail from the mask target.
- [x] Clear the raster target when a vector-mask thumbnail is clicked.
- [x] Update `tst_layers_panel::maskEditTargetActivation` and
  `tst_mask_edit::colorsFollowTheEditTarget` to the new behaviour.

## 3. Group expansion survives a structural change

- [x] Add `LayersModel::namePathForPath` / `indexForNamePath`.
- [x] Remap expansion by name chain in `LayersPanel::refresh`.
- [x] Qt checks: `tst_layers_panel_fixes::addingLayerKeepsGroupExpanded` and
  `dropIntoOpenGroupKeepsExpanded`.

## 4. Group icon

- [x] Skip the black outline for group rows.
- [x] Grow the folder glyph from half to three quarters of the thumbnail.
- [x] Qt checks: `tst_layers_panel_fixes::groupIconHasNoOutline` and
  `groupIconIsLarger`.

## 5. Thumbnails fill the row

- [x] Add the full-height `thumbSlot` and use it for image/mask/vector thumbs.
- [x] Fix `letterboxedThumb` for a non-square box.
- [x] Qt check: `tst_layers_panel_fixes::thumbnailFillsRow`.

## 6. Chain glyph

- [x] Draw the link glyph two pixels larger and rotated 90°.
- [x] Qt check: `tst_layers_panel_fixes::linkGlyphRotatedAndLarger`.

## 7. Combo affordances

- [x] Add the `assets/ui/combo-caret{,-disabled}.svg` assets and the
  `QComboBox::down-arrow` stylesheet rule.
- [x] Keep a focused header control's focus across a refresh in `syncControls`.
- [x] Qt check: `tst_layers_panel_fixes::blendComboArrowAndKeys` (caret shape +
  focus retained across changes).

## 8. Blend-change history label

- [x] Add `BlendMode::name` in `pictura-core`.
- [x] Label the blend bridge's undo state `Blending Change: <name>`.
- [x] Rust check: `pictura-core::blend_names_are_distinct_and_non_empty`.
- [x] Qt check: `tst_layers_panel_fixes::blendChangeHistoryLabel`.

## Verification

- [x] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo nextest run -p pictura-render` (the local ImageMagick fixture
  mismatch is pre-existing).
- [x] `ctest -R '^tst_'` and the headless self-test.
- [x] `bash scripts/verify-fast.sh`.
- [x] `openspec validate --all --strict`.
