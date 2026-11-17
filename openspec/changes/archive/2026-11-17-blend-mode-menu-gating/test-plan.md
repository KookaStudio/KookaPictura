# Test plan

One row per scenario in the two delta specs, plus the supporting checks that
prove the cross-cutting behavior. Coverage names an automated test
(`<crate>::<fn>` for Rust, `tst_<suite>::<case>` for Qt Test) or
`manual: <reason>`.

Qt tests that convert documents include `api/document_bridge.h`
(`convert_image_mode`, `convert_to_indexed`, `create_document`). The restore
case converts a Lab document back to RGB; a 32→8-bit HDR conversion flattens
the layer tree, so it cannot preserve a layer's stored mode.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `blend_list_engine` | Enumerate the engine layer-mode list | 27 entries in layer-mode order, keys round-trip, names are the display names | `pictura-app::blend_mode_rows_follow_layer_modes`, `tst_blend_mode_list::layerCombosFollowEngineList` |
| `blend_list_combos` | Populate the Layers panel combo and Mode filter, the New Layer dialog's Mode combo and the Layer Style blend combo | Offered engine names in engine order; Layers panel/filter/New Layer item data is the engine key, Layer Style item data is the engine index | `tst_blend_mode_list::layerCombosFollowEngineList`, `tst_blend_mode_list::layerStyleComboIsEngineIndexed` |
| `blend_list_set` | On an 8-bit RGB document pick entry at display index *i* in the Layers panel combo | The active layer's blend key is that entry's engine key item data | `tst_blend_mode_list::comboIndexSetsEngineKey` |
| `bma_hdr` | Create a 32-bit RGB document with a layer; inspect the Layers panel combo, the New Layer dialog and the Layer Style combos | Exactly the 15 documented HDR modes are selectable and no other mode is selectable; an unavailable stored mode is present and disabled | `pictura-core::blend_availability_32_bit`, `tst_blend_mode_list::layersComboOffersHdrSubset`, `tst_blend_mode_list::newLayerDialogFollowsGate`, `tst_blend_mode_list::layerStyleCombosFollowGate` |
| `bma_lab` | Create an 8-bit RGB document with a layer, convert it to Lab (`convert_image_mode`; include `api/document_bridge.h`), refresh; inspect the same controls | 19 selectable modes; Color Dodge, Color Burn, Darken, Lighten, Difference, Exclusion, Subtract and Divide are not selectable; an unavailable stored mode is present and disabled | `pictura-core::blend_availability_lab`, `tst_blend_mode_list::layersComboFollowsLab`, `tst_blend_mode_list::newLayerDialogFollowsGate`, `tst_blend_mode_list::layerStyleCombosFollowGate` |
| `bma_bitmap` | Query the availability result for Bitmap and Indexed documents; convert a document to Indexed (`convert_to_indexed`; include `api/document_bridge.h`) and inspect the populated Layers panel combo | Normal is the only available and selectable mode | `pictura-core::blend_availability_bitmap_indexed`, `tst_blend_mode_list::layersComboFollowsIndexed`, `pictura-app::allowed_blend_mode_keys_follow_mode_and_depth` |
| `bma_controls` | For a Lab and a 32-bit document, inspect the Layers panel, New Layer (including the `group=true` New Group case), Layer Style layer and effect combos, and the Fade Mode menu | Every control offers exactly the available modes for selection; an unavailable stored or default mode is present but disabled; Fade presents every excluded mode disabled | `tst_blend_mode_list::layersComboFollowsLab`, `tst_blend_mode_list::newLayerDialogFollowsGate`, `tst_blend_mode_list::layerStyleCombosFollowGate`, `tst_fade::labDisablesRestrictedModes`, `tst_fade::hdrDisablesRestrictedModes` |
| `bma_conflict` | Create a 32-bit RGB document first, add a layer, set it to Screen, refresh the panel; open Layer Style; choose Multiply | Layers panel: key stays `scrn`, then choosing Multiply writes `mul `. Layer Style: `blending.mode` stays Screen's engine index, then choosing Multiply writes Multiply's engine index | `tst_blend_mode_list::layersComboKeepsUnsupportedCurrent`, `tst_blend_mode_list::layerStyleKeepsUnsupportedCurrent` |
| `bma_restore` | Convert the Lab document back to RGB with `convert_image_mode` and refresh | The combo offers all 27 modes and the stored mode is selectable again | `tst_blend_mode_list::layersComboRestoresAllModes` |
| The dialog opens on the retained values | Apply a 40 % Multiply fade and reopen the dialog | Opacity reads 40, Mode reads Multiply, Preview is checked | `tst_fade::dialogOpensOnRetainedValues` |
| Preview updates the canvas without history | Drag Opacity with Preview on | The canvas changes and the history count is unchanged | `tst_fade::previewUpdatesCanvasWithoutHistory` |
| Cancel restores the pre-dialog pixels | Change Opacity and cancel | The document is bit-identical and history is unchanged | `tst_fade::cancelRestoresPixels` |
| OK commits one Fade state | Change the mode and accept | Exactly one `Fade <effect>` state is added with the faded result | `tst_fade::okCommitsOneState` |
| Reopening recomputes from the effect output | Apply 50 % Normal, reopen, apply 25 % Normal | The result is 25 % of the original effect output | `tst_fade::reopeningRecomputesFromEffectOutput` |
| Lab disables the restricted modes | Open Fade on a Lab document and inspect the mode items | Color Dodge, Color Burn, Darken, Lighten, Difference, Exclusion, Subtract and Divide are visible but disabled | `tst_fade::labDisablesRestrictedModes` |
| `fade_32` | Create a 32-bit RGB document, apply an effect, open Fade and inspect the mode items | The 12 modes outside the HDR list are visible but disabled; the 15 HDR modes are selectable | `tst_fade::hdrDisablesRestrictedModes` |
| An 8-bit RGB document offers all modes | Open Fade on an 8-bit RGB document and inspect the mode items | All 27 modes are enabled | `tst_fade::eightBitOffersAllModes` |

## Supporting tests

These checks do not map to a new scenario; they pin the cross-cutting behavior
the scenarios above rely on.

- A group row keeps the Normal fallback and never gains a disabled `pass` entry — `tst_blend_mode_list::layersComboLeavesGroupPassThrough`
- A group's Layer Style `blending.mode` is NaN, so the combo stays at its default and appends nothing — `tst_layer_style::groupBlendingOptions`
- A malformed Lab+32-bit document yields the intersection of the two restriction sets — `pictura-core::blend_availability_lab_32_bit_intersection`
- The Layer Style blend combo carries the engine index in item data — `tst_blend_mode_list::layerStyleComboIsEngineIndexed`
