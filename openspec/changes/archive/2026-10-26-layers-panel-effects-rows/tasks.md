# Tasks

## 1. Tests first

- [x] Add `tst_layers_effects` (new suite) with `nestedEffectRows`,
  `effectsEyeTogglesMaster`, `effectEyeTogglesOne`, `newStyleStartsExpanded`,
  `unstyledLayerHasNoGroup`, `doubleClickEffectOpensItsPage`,
  `doubleClickBadgeOpensBlendingOptions`, `rowMenuListsLayerStyleCommands`, and
  `disableLayerEffectsIsOneState`.
- [x] Register `tst_layers_effects` in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] Extend `tst_layers_snapshot` for the enabled-effects and master-switch
  projection.

## 2. Bridge

- [x] Add `style_effects_on` and `style_visible` to the Rust `LayerRow` and fill
  them in `layers_snapshot`.
- [x] Convert them in `layers_api.{h,cpp}`.

## 3. Panel rows

- [x] Build the synthetic `Effects` group and effect rows
  (`LayersPanel::appendEffectRows`).
- [x] Carry the snapshot's `styleEffectsOn`/`styleVisible` into the synthetic
  rows' eye state (no new model role).
- [x] Paint the `layers.fx` glyph for the two new kinds.
- [x] Auto-expand a newly styled layer when **Expand New Effects** is on.
- [x] Exclude `@fx` paths from the layer target and selection.

## 4. Interactions

- [x] Eye clicks on the group/effect rows toggle `fx.visible` / `<effect>.on` as
  one state.
- [x] The `fx` badge click toggles the group's expansion.
- [x] Double-click opens the Layer Style dialog on the right page.
- [x] Right-click opens the CS6 layer-style menu.
- [x] Add the dialog's effect title/keys table and the `openLayerStyle` effect
  argument.

## 5. Docs and verification

- [x] Update `ROADMAP.md` and `docs/dev/STATE.md`.
- [x] Run `openspec validate --all --strict`, the focused Qt suites, and archive
  the change.
