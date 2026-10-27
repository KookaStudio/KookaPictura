# Test plan

Every scenario in the delta spec has a row. CI runs the Qt suites; no local
build is required for this change.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_fx_rows` | Style a pixel layer with a drop shadow and refresh the panel | The layer row is followed by a synthetic `Effects` row and a `Drop Shadow` row | `tst_layers_effects::nestedEffectRows` |
| `lpr_fx_group_eye` | Click the `Effects` row's eye | `fx.visible` is false, the composite is unstyled, the eye reads hidden, one state | `tst_layers_effects::effectsEyeTogglesMaster` |
| `lpr_fx_effect_eye` | Click a `Drop Shadow` effect row's eye | `dropShadow.on` flips, the composite changes, the row stays listed | `tst_layers_effects::effectEyeTogglesOne` |
| `lpr_fx_expand_new` | Add a style with **Expand New Effects** on | The group and its effect rows are shown expanded | `tst_layers_effects::newStyleStartsExpanded` |
| `lpr_fx_no_group` | Show an unstyled layer | No `Effects` row follows it | `tst_layers_effects::unstyledLayerHasNoGroup` |
| `lpr_fx_dblclick_effect` | Double-click a `Drop Shadow` effect row | The Layer Style dialog opens on the Drop Shadow page | `tst_layers_effects::doubleClickEffectOpensItsPage` |
| `lpr_fx_dblclick_badge` | Double-click a styled layer's `fx` badge | The Layer Style dialog opens on Blending Options | `tst_layers_effects::doubleClickBadgeOpensBlendingOptions` |
| `lpr_fx_menu` | Right-click the `fx` badge / an effects row | Disable Layer Effects, Blending Options…, ten effect entries, Copy/Paste/Clear, disabled Global Light…/Create Layer, Hide All Effects, Scale Effects… | `tst_layers_effects::rowMenuListsLayerStyleCommands` |
| `lpr_fx_disable` | Choose Disable Layer Effects | `fx.visible` is false, composite unstyled, exactly one state | `tst_layers_effects::disableLayerEffectsIsOneState` |
