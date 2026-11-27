# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tpp_toggle` | Trigger `Window > Panels > Tool Presets` twice | The panel shows and the action is checked, then the panel hides and the action is unchecked | `tst_tool_presets_panel::panelTogglesFromWindowMenu` |
| `tpp_painting_workspace` | Apply the Painting workspace on a fresh session | The Tool Presets panel is hosted | `tst_tool_presets_panel::paintingWorkspaceHostsPanel` |
| `tpp_create` | Configure the Brush, create a named preset, inspect the Tools list | A row with the name, `brush`, and the current option values exists and the panel lists it | `tst_tool_presets_panel::createCapturesActiveToolAndOptions` |
| `tpp_apply_switches` | Create a Move preset, switch to Brush, apply the Move preset | Move becomes active and its captured options are restored | `tst_tool_presets_panel::applySwitchesToolAndRestoresOptions` |
| `tpp_apply_type` | Create a type preset with a size and justification, change them, apply the preset | The Type options report the captured size and justification | `tst_tool_presets_panel::typePresetRestoresTypeOptions` |
| `tpp_apply_no_history` | Apply a preset with a document open | The history count and dirty state are unchanged | `tst_tool_presets_panel::applyingPresetAddsNoHistory` |
| `tpp_filter` | Turn Current Tool Only on with presets for two tools | Only the active tool's presets are listed; an active tool with none yields an empty list | `tst_tool_presets_panel::currentToolOnlyFilters` |
| `tpp_sort` | Create presets out of tool order and turn Sort By Tool on | Rows group by owning tool while preserving within-tool order | `tst_tool_presets_panel::sortByToolGroups` |
| `tpp_display` | Switch Text Only, Small List, and Large List | The same rows remain with text-only and the corresponding icon sizes | `tst_tool_presets_panel::displayModesChangeList` |
| `tpp_rename_delete` | Rename then delete a preset and read the picker and Manager | Both consumers show the new name and then no deleted row | `tst_tool_presets_panel::renameAndDeleteReachPickerAndManager` |
| `tpp_reorder_persist` | Reorder rows in the panel, then construct a fresh store | The new order is read back from the current list | `tst_tool_presets_panel::reorderPersistsAcrossStoreReload` |
| `tpp_load_replace_reset_save` | Save the current Tools list, reset it, then load the saved file | Save writes `.kpset`, reset restores defaults, load appends the saved rows | `tst_tool_presets_panel::loadReplaceResetSaveTools` |
| `tpp_discovery` | Place a Tools library in the discovery folder and restart | The panel menu offers the library | `tst_tool_presets_panel::discoversToolLibraries` |
| `tpp_invalid` | Import truncated bytes with Replace | The failure is reported and the current list is unchanged | `tst_tool_presets_panel::invalidLibraryKeepsList` |
| `tpp_picker` | Create a Move preset and open Move's options-bar picker | The preset is listed and applying it restores the captured options | `tst_tool_presets_panel::optionsBarPickerListsEveryTool` |
| `tpp_native_round_trip` | Create a preset with several option groups, save, read back, apply | Every captured value is restored | `tst_tool_presets_panel::nativeToolPresetRoundTripsOptions` |
| `tpp_unknown_options` | Add an unknown field to a native preset's opaque snapshot and re-save | The field is still present | `tst_tool_presets_panel::unknownOptionFieldsSurviveResave`, `pictura-codec::native_tool_preset_preserves_opaque_options` |
| `preset_ui_apply` | Import one usable entry of each managed type and apply it through its owning control | Each color, stroke, gradient, effect, pattern, contour, shape, or tool option reflects the imported payload | `tst_preset_manager::appliesImportedSwatchBrushAndToolOptions`, `tst_preset_manager::appliesImportedGradientStyleAndContour`, `tst_preset_manager::appliesImportedPatternsAndCustomShape` |
| `preset_ui_snapshot` | Edit or remove the selected entry while a stroke holds its captured settings or after a document embeds an asset | The stroke and reopened document keep their previous appearance and no History state is added | `tst_preset_manager::libraryEditsPreserveStrokeAndDocumentContent` |
| `preset_ui_tools_picker` | Open a non-Brush tool's picker with a matching preset | The preset is listed and applying changes that tool's options | `tst_tool_presets_panel::optionsBarPickerListsEveryTool` |
| `preset_ui_tools_library` | Run Load, Replace, Reset, and Save on the Manager's Tools page | The Tools list is appended, replaced, reset, or written natively | `tst_tool_presets_panel::presetManagerToolsActions` |
| `preset_codec_tool_native` | Decode and re-encode a native Tools set with an opaque snapshot | The tool identifier, mapped values, and snapshot are unchanged | `pictura-codec::native_tool_preset_preserves_opaque_options` |
| `preset_codec_tool_opaque` | Decode a native Tools entry with arbitrary opaque bytes | The bytes are preserved verbatim and a malformed snapshot does not invalidate the entry | `pictura-codec::native_tool_preset_keeps_unknown_opaque_bytes` |
