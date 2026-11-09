# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pat_builtin_available` | Call `builtin_patterns()` and list the available ids for a document with no `Patt` block | At least one built-in pattern is listed and the editor is not empty | `pictura-codec::builtin_patterns_are_available`, `tst_fill_layer_editor::patternBuiltinsOnBlankDocument` |
| `pat_builtin_embed` | Encode a built-in, append it to a document, then create a pattern fill that references it | The document decodes the pattern, the fill composites the tile, and the document round-trips | `pictura-codec::builtin_patterns_encode_into_a_document`, `tst_fill_layer_editor::patternBuiltinFillEmbedsAndRenders` |
| `fle_pattern_apply` | Open the pattern fixture, add a pattern fill with the first document pattern, scale 50, and link | The layer's `PtFl` carries the id, scale, and link flag | `tst_fill_layer_editor::patternApply` |
| `fle_pattern_snap` | Open the editor with a non-zero origin and click Snap To Origin | The payload's origin becomes `(0, 0)` | `tst_fill_layer_editor::patternSnapToOrigin` |
| `fle_pattern_builtins` | Open the Pattern Fill editor on a blank document | It lists the built-ins and is not empty; creating with one embeds it | `tst_fill_layer_editor::patternBuiltinsOnBlankDocument`, `tst_fill_layer_editor::patternBuiltinFillEmbedsAndRenders` |
| `fle_pattern_empty` | Configure a view with no document and no built-in patterns and open the editor | The empty state is reported and creation is unavailable | manual: the built-in library is always available, so this is a defensive fallback with no reachable production path |
| The menu lists every adjustment kind | Open the Layers-panel New Fill/Adjustment menu | Every adjustment kind is present and creates its layer | `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue` |
| Pattern stays disabled until authoring exists | Open the menu when neither the document nor the built-in library provides a pattern | Pattern… is present, disabled, and carries the no-patterns reason | manual: the built-in library is always available; this is a defensive fallback with no reachable production path |
| `lpr_pattern_entry` | Open the Layers-panel menu on a blank document | Pattern… is present and enabled | `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue` |
| `lpr_layer_fill_menu` | Build the `Layer > New Fill Layer` submenu on a blank document | Pattern… is enabled, matching the panel menu | `tst_fill_layer_editor::layerFillMenuPatternGating` |
| Solid Color icon opens the solid-color flow | Click the Solid Color icon on a blank document | A Solid Color fill layer is created and its editor opens | `tst_adjustments_panel::solidColorIconDispatchesFillCommand` |
| Pattern icon is inert | Inspect the Pattern icon with no document open or no pattern library | The icon is disabled with the no-patterns reason | `tst_adjustments_panel::noDocumentDisablesGrid` |
| `adj_pattern_icon` | Click the Pattern icon on a blank document and reject the editor | A default Pattern fill is created, the editor opens, and cancelling keeps the layer | `tst_adjustments_panel::patternIconCreatesDefaultFill` |
| No document disables the grid | Inspect the grid with no document open | Every entry is disabled | `tst_adjustments_panel::noDocumentDisablesGrid` |
| A document enables the grid | Inspect the grid with a blank document open | Every entry, including Pattern, is enabled | `tst_adjustments_panel::documentEnablesGrid` |
