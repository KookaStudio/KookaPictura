# Refresh audit

Rule: a handler's `refresh();` is deleted when every successful outcome its
command can commit notifies. A handler whose command can commit an outcome that
does not notify, commits no edit, or changes only transient state keeps its
`refresh();`.

## `frame_menus.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| File > Place | `place_smart_object` / `place_image` | `Structural` | delete |
| Image > Rotate 90° CW/CCW/180, Flip H/V | `rotate_doc` / `flip_doc` | `Full` | delete |
| Layer > New Layer / New Group / Group from Layers | dialogs -> `new_layer_dialog` / `new_group_dialog` / `group_from_layers_dialog` | `Structural` on accept, no edit on cancel | delete |
| Layer > Duplicate Layer | `duplicateSelection` -> `duplicate_layers` | `Structural` | delete |
| Layer > Group / Ungroup Layers | `groupSelection` / `ungroupSelection` | `Structural` | delete |
| Layer > Merge Layers / Merge Visible | `merge_layers` / `merge_visible` | `Structural` | delete |
| Layer > Create / Release Clipping Mask | `clipping_create` / `clipping_release` | `Full` | delete |
| Layer > Merge Clipping Mask | `merge_clipping_mask` | `Structural` | delete |
| Layer > Add Layer Mask family | `layer_mask_add` | `Full` | delete |
| Layer > Delete / Apply / Enable / Disable / Link / Unlink Mask | `layer_mask_*` | `Full` | delete |
| Layer > Flatten Image | `flatten_image` | `Structural` | delete |
| Layer > Layer from Background / Background from Layer | `layer_from_background` / `background_from_layer` | `Structural` | delete |
| Layer > Layer via Copy / Cut | `layer_via_copy` / `layer_via_cut` | `Structural` | delete |
| Layer > Delete Layer | `deleteSelection` -> `delete_layers` | `Structural` | delete |
| Layer > Delete Hidden Layers | `delete_hidden_layers` | `Structural` | delete |
| Layer > Hide Layers | `hide_layers` | `Full` | delete |
| Layer > New Fill Layer (Solid Color / Gradient) | `add_solid_fill` / `add_gradient_fill` | `Structural` | delete |
| Layer > Rasterize Fill Content / Layer / Type / All Layers | `rasterize_*` | `Structural` | delete |
| Layer > Smart Object > Convert to Smart Object | `convert_to_smart_object` | `Structural` | delete |
| Filter > Convert for Smart Filters | `convert_for_smart_filters` | `Structural` | delete |
| Layer > Smart Object > Rasterize | `rasterize_smart_object` | `Structural` | delete |
| Layer > Select Similar / Select Linked | `select_similar` / `select_linked` | no edit | keep |
| Layer > Smart Object > Edit Contents | `editSmartObjectContents` opens an editor document | no edit on the active view | keep |
| Filter > Pictura Raw | `apply_pictura_raw_filter` | mixed destructive / retained smart filter | keep |
| View > Free Pan / Snap Canvas to Workspace | view state | no document edit | keep (`registry_->refresh`) |
| `selectLayerPath` helper | panel selection | no document edit | keep (`layersPanel_->refresh`) |

## `frame_menus_layer_ops.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Layer > Arrange Front/Forward/Backward/Back | `arrange_layer_run` | `Structural` | delete |
| Layer > Arrange > Reverse | `reverse_layers_run` | `Structural` | delete |
| Layer > Stamp Visible / Stamp Selected | `stamp_*_run` | `Structural` | delete |
| Layer > Lock Layers family | `set_layers_lock` -> `batch_property` | `Meta` | delete |
| Layer > Lock All Layers in Group | `lock_group_layers_run` | `Full` | delete |
| Layer > Rasterize Shape | `shape_rasterize` | `Structural` | delete |
| Layer > Paste Shape Attributes | `shape_paste_attributes` | `Full` | delete |
| Layer > Vector Mask add family | `vector_mask_add` | `Full` | delete |
| Layer > Vector Mask delete/enable/disable/link/unlink/rasterize | `vector_mask_*` | `Full` | delete |

## `frame_menus_layer_style.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Layer Style dialog pages | `layer_style_commit` | `Record` (already-presented preview) | keep |
| Copy Layer Style | `layer_style_copy` | no edit | keep |
| Paste / Clear / Scale / Show All / Hide All | `layer_style_*` -> `batch_changed` / `set_all_effects_visible` | `Full` | delete |

## `frame_menus_align.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Layer > Align / Align Layers to Selection | `align_apply` -> `batch_changed` | `Full` | delete |
| Layer > Distribute | `distribute_apply` -> `batch_changed` | `Full` | delete |

## `frame_menus_type.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Type > Convert to Point / Paragraph Text | `type_api::convert` | `Structural` | delete |

## `frame_menus_smart_object.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Smart Object > Reset Transform | `smart_object_reset_transform` | `Structural` | delete |
| Smart Object > Convert to Layers | `smart_object_convert_to_layers` | `Structural` | delete |
| Smart Object > New via Copy | `smart_object_new_via_copy` | `Structural` | delete |
| Smart Object > Replace Contents | `replace_smart_object_contents` | `Structural` | delete |
| Layer > Smart Filters > Clear | `clear_layer_smart_filters` | `Full` | delete |
| Smart Object > Edit Contents | `editSmartObjectContents` | no edit on the active view | keep (no local refresh) |

## `frame_menus_image.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Image > Crop | `crop` | `Full` | delete |
| Image > Trim | `trim_image` -> `finish` | `Full` | delete |
| Image > Image Size | `image_size_apply` | `Full` / `Meta` | delete |
| Image > Canvas Size | `canvas_size_apply` | `Full` | delete |
| Image > Rotate Arbitrary | `rotate_document_arbitrary` | `Full` | delete |
| Image > Duplicate | `duplicate_into` | `Full` on the new view | no local refresh |

## `frame_menus_select.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Select > Reselect / Inverse | `reselect` / `invert_selection` | `Meta` | delete |
| Select > Modify family / Grow / Similar | `modify_selection` / `grow_selection` / `similar_selection` | `Meta` | delete |
| Select > Save Selection | `save_selection` | `Meta` | delete |
| Select > Load Selection | `load_selection` | `Meta` | delete |
| Select > Color Range | `color_range_apply` | `Meta` | delete |
| Select > Refine Edge | `refine_edge_apply` | `Full` | delete |
| Select > All Layers / Deselect Layers / Similar Layers | layer-target state | no edit | keep |

## `frame_menus_matting.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Layer > Matting > Remove Black / White Matte | `layer_remove_*_matte` | `Full` | delete |
| Layer > Matting > Defringe | `layer_defringe` | `Full` | delete |

## `frame_menus_edit.cpp`

| handler | command / bridge | outcome | decision |
|---|---|---|---|
| Edit > Copy / Cut / Clear / Copy Merged | clipboard | no document edit or mixed | keep |
| Edit > Paste family | clipboard paste | mixed | keep |
| Edit > Fill / Stroke | fill / stroke engine | mixed | keep |
| Edit > Purge Undo / Histories / All | history maintenance | no document edit | keep |
| Delete key, mask branch | fill / mask delete | mixed | keep |
| Delete key, layer branch | `deleteSelection` | `Structural` | delete |
| Layer copy from another document | `copy_layer_from_document` | `Structural` | delete |
| Edit > Undo / Redo tool path | tool session | no document edit | keep |

The Edit > Undo label provider names the state at the current history cursor
(and Redo the next state after an undo). That changes the command's text, not
whether its handler refreshes, so it does not alter any row above.
