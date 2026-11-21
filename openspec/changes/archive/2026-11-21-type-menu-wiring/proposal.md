# Proposal

## Why

The Type menu still shows roughly 25 disabled rows. Warp Text, the point/paragraph conversion, and Replace All Missing Fonts are wired, but the structural commands and the option rows are not. `docs/dev/doc-code-parity-gaps.md` tracks the gap as UI-6 (#502), and `ROADMAP.md` names work path, convert to shape, and spellcheck.

The target is a CC-shaped menu built on the CS6 contract: implement the rows that map onto the existing engine, keep Adobe-specific and ML surfaces out, and track them separately. Google Fonts is follow-up #1442; Check Spelling is follow-up #1443.

## What Changes

- Implement `Type > Create Work Path` and `Type > Convert to Shape` from a live type layer, including point, paragraph, warped, on-path, and area type. Area type traces its laid-out glyphs without the render-time clip (documented ceiling).
- Create Work Path replaces the document's single work path and leaves the type layer untouched.
- Convert to Shape replaces the type layer in place with a shape layer carrying the same name, outline, effects, and fill colour.
- Wire `Type > Anti-Alias` (5 values), `Type > Orientation` (2), `Type > Rasterize Type Layer`, `Type > Update All Text Layers`, `Type > Language Options` (Default / East Asian / Middle Eastern), `Type > Font Preview Size` (6 values), `Type > Paste Lorem Ipsum`, and `Type > Load/Save Default Type Styles`.
- Add `Type > Panels > Glyphs` and `Type > OpenType` for the 10 modelled OpenType features.
- Add `Edit > Find And Replace Text…` with current-layer or all-layers scope, Forward, Case Sensitive, Whole Word Only, case/whole-word matching, and one history state per Replace All.
- Remove the doc-divergent Type rows: `Check Spelling…` and `Find And Replace Text…` (Edit-only per `docs/02-ui-ux/menus.md`) and `Extrude to 3D` (removed from modern Photoshop CC).
- Update the command-registry spec's stale statement that the Type rasterize entry has no handler.
- Update `ROADMAP.md` and the parity register; record the area-outline and find/replace case-folding ceilings in `docs/03-tools/type-tools.md`.

## Capabilities

### New Capabilities

- `tools/type-outline`: Create Work Path and Convert to Shape from a type layer's glyph outlines, including warped, on-path, and area type, the outline-font gate, and the one-state history contract.
- `ui/type-menu`: the remaining Type menu rows, their check/enable states, Paste Lorem Ipsum, Font Preview Size, the Glyphs and OpenType rows, Load/Save Default Type Styles, and the removal of the doc-divergent rows.
- `ui/find-and-replace-text`: the `Edit > Find And Replace Text…` dialog, its matching options, scope, and history behaviour.
- `tools/type-default-styles`: saving the active type layer's character and paragraph defaults and loading them onto a type layer.

### Modified Capabilities

- `document/command-registry`: the Type rasterize entry now has a handler; Shape, Vector Mask, Smart Object, Video, and 3D stay disabled.
- `ui/font-family-picker`: the preview sample follows `Type > Font Preview Size`; `None` omits the sample.
- `ui/layers-panel`: the Type row enables Create Work Path and Convert to Shape for an outline face and keeps them disabled for an outline-less face.

## Impact

- `crates/pictura-render`: a new `type_outline` module, a type-to-shape document operation, and outline/conversion tests.
- `crates/pictura-app`: `type_tools` bridge and `type_api` additions; `menu_type.cpp`, `menu_edit.cpp`, `layers_panel_type_menu.cpp`, `font_combo.cpp`, the typed preferences store, `frame`, and a new `find_replace_dialog`.
- `crates/pictura-core`: the pure find/replace matcher.
- `crates/pictura-script`: catalogue rows, checkable/implemented flags, golden catalogue and const-name updates.
- Qt suites: new `tst_type_menu` and `tst_find_replace`; updates to `tst_layers_row_menu` and catalogue assertions.
- No new dependencies. SCOWL spelling (#1443) and the Google Fonts browser (#1442) stay out of this change.
