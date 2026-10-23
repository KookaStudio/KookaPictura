# Test Plan — actions-bundle

Every row maps one spec scenario to an automated test or a justified manual
check. Test names are the planned functions; tasks.md creates the ones that do
not exist yet.

## `codec/actions-file`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `af_atn_read` | Build a synthetic v16 `.atn` with one set, two actions, and steps; read it | Every documented field decodes, including `hasDescriptor` and the byte-string display name | `pictura-codec::af_atn_read` |
| `af_atn_no_magic` | Read a valid file whose first bytes are the version integer | The set loads; no `8BAC` check runs | `pictura-codec::af_atn_no_magic` |
| `af_atn_version` | Read files with versions 12, 15, and 17 | Each returns a named version error and no set | `pictura-codec::af_atn_version` |
| `af_atn_trailing` | Append bytes after the declared set and read/write | The trailing bytes are preserved unchanged | `pictura-codec::af_atn_trailing` |
| `af_atn_dialog_options` | Read and write steps with dialog options 0–4 | 0–3 survive; 4 is written as 0 | `pictura-codec::af_atn_dialog_options` |
| `af_atn_roundtrip` | Write a supported set and read it back | The decoded structure equals the source | `pictura-codec::af_atn_roundtrip` |
| `af_atn_id_form` | Encode a string id and a code with the same characters | Bytes and decoded forms differ but each round-trips | `pictura-codec::af_atn_id_form` |
| `af_atn_order` | Encode duplicate keys and unsorted items | Written bytes and read order match the source | `pictura-codec::af_atn_order` |
| `af_atn_unknown_save` | Read a set with an unknown event identifier but a decodable descriptor; write it | The identifier and descriptor are re-emitted unchanged | `pictura-codec::af_atn_unknown_save` |
| `af_psp_roundtrip` | Write and read a two-set palette | Both sets and their order are preserved | `pictura-codec::af_psp_roundtrip` |
| `af_psp_count` | Declare more sets than the bytes contain | A truncated-palette error is returned with the decoded prefix | `pictura-codec::af_psp_count` |
| `af_psp_partial` | Truncate a two-set palette after the first set | The first set is returned with an error naming the offset | `pictura-codec::af_psp_partial` |
| `af_psp_version` | Read a palette whose version is not 16 | A named version error is returned and no set loads | `pictura-codec::af_psp_version` |
| `af_desc_values` | Encode every supported descriptor value, including `ObAr`, `UnFl`, and `Pth ` | Decode/encode reproduces values and bytes | `pictura-codec::af_desc_values` |
| `af_desc_refs` | Encode every reference form | Decode/encode reproduces forms and bytes | `pictura-codec::af_desc_refs` |
| `af_desc_idnt_class` | Decode `Idnt` and `indx` from a v16 `.atn` | The desired class precedes the numeric value | `pictura-codec::af_desc_idnt_class` |
| `af_desc_unknown` | Decode an unrecognized OSType | A named error is returned; no guessed length | `pictura-codec::af_desc_unknown` |
| `af_malformed_truncated` | Cut a file inside a string, descriptor, and action | Typed truncation errors; no panic | `pictura-codec::af_malformed_truncated` |
| `af_malformed_count` | Encode a count above the bound | A typed error returns before allocation | `pictura-codec::af_malformed_count` |
| `ae_load_failure_keeps_library` | Load a malformed file over an existing library | The library is unchanged; the error names the source and offset | `pictura-script::ae_load_failure_keeps_library` |

## `document/action-engine`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ae_model_roundtrip` | Save and load a set with actions and steps | Order, names, flags, keys, and colors match | `pictura-script::ae_model_roundtrip` |
| `ae_model_empty` | Load/save/play an empty set and action | No error and no history state | `pictura-script::ae_model_empty` |
| `ae_record_dialog` | Record a bus-level dialog command with changed values | One step contains those values | `tst_command_bus::ae_record_dialog` |
| `ae_record_empty` | Record a parameterless command | One step with no parameters is appended | `tst_command_bus::ae_record_empty` |
| `ae_record_excluded` | Execute a non-recordable command while recording | No step is appended | `tst_command_bus::ae_record_excluded` |
| `ae_record_coverage` | Run the coverage check with an unclassified id | The check fails and names the command | `pictura-script::ae_record_coverage` |
| `ae_record_lifecycle` | Start, record one step, stop, start again, record | Both steps are retained in order | `pictura-script::ae_record_lifecycle` |
| `ae_record_refused` | Record a command the engine refuses | No step, no document change | `tst_command_bus::ae_record_refused` |
| `ae_play_order` | Play an action with one disabled step | Enabled steps run in order; disabled skipped | `pictura-script::ae_play_order` |
| `ap_play_set` | Select a set and Play | Its included actions play in library order | `tst_actions_panel::ap_play_set` |
| `ae_play_modal` | Play a modal step and confirm changed values | The command uses the entered values | `tst_actions_panel::ae_play_modal` |
| `ae_play_recorded` | Play a dialog-off step | The command uses recorded values with no dialog | `tst_actions_panel::ae_play_recorded` |
| `ae_play_specific` | Play a specific-dialog step whose modal control is off | The command's dialog opens and waits for input | `tst_actions_panel::ae_play_specific` |
| `ae_play_stepwise` | Play two steps at Step By Step speed | The canvas is painted between steps | manual: visual redraw between steps; playback order is covered by pictura-script::ae_play_order |
| `ae_play_stop` | Play a Stop step with Continue | The message appears and Continue resumes | `tst_actions_panel::ae_play_stop` |
| `ae_play_menu_item` | Play an Insert Menu Item step | The command runs; a dialog pauses playback | `tst_actions_panel::ae_play_menu_item` |
| `ae_play_path` | Play an Insert Path step | The work path becomes the recorded path | `tst_actions_panel::ae_play_path` |
| `ae_play_recursion` | Play an A→B→A cycle | Playback aborts with a recursion error | `pictura-script::ae_play_recursion` |
| `ae_play_cancel` | Cancel after two steps | Two steps remain applied with history | `tst_actions_panel::ae_play_cancel` |
| `ae_play_error` | Play a step with an invalid target | Playback stops and names the step | `pictura-script::ae_play_error` |
| `ae_unsupported_play` | Play a step with an unknown event id | Playback stops with the event and step index | `pictura-script::ae_unsupported_play` |
| `ae_key_precedence` | Assign a command's key to an action and press it | The action plays; the command does not | `tst_actions_panel::ae_key_precedence` |
| `ae_key_unique` | Assign the same key to two actions | The second assignment is refused | `tst_actions_panel::ae_key_unique` |
| `ae_key_restore` | Delete the shadowing action, then press the key | The command runs | `tst_actions_panel::ae_key_restore` |
| `ae_reset_replace` | Reset with OK | Only the default set remains | `pictura-script::ae_reset_replace` |
| `ae_reset_append` | Reset with Append, including when a default set already exists | Existing sets remain; the default set is appended with a unique name | `pictura-script::ae_reset_append` |
| `ae_clear_all` | Clear All | No sets remain | `pictura-script::ae_clear_all` |
| `ae_default_provenance` | Inspect the default set | Only the four actions; no Adobe preset name | `pictura-script::ae_default_provenance` |
| `ae_tool_replay` | Record a brush stroke and play it | Same geometry; one history state | `tst_actions_panel::ae_tool_replay` |
| `ae_tool_settings` | Record with non-default settings and change them | Playback uses captured settings | `tst_actions_panel::ae_tool_settings` |
| `ae_tool_off` | Record without enabling the option | No tool step | `tst_actions_panel::ae_tool_off` |
| `ae_tool_limit` | Record a stroke over the sample/payload bound | The step is refused and the action is unchanged | `tst_actions_panel::ae_tool_limit` |
| `ae_conditional_branch` | Play a conditional with true and false conditions | Only the matching action plays | `pictura-script::ae_conditional_branch` |
| `ae_conditional_invalid` | Insert a conditional with both None | The dialog refuses | `tst_actions_panel::ae_conditional_invalid` |
| `ae_conditional_invalid_play` | Load and play a conditional with both None | Playback stops with a named error | `pictura-script::ae_conditional_invalid_play` |
| `ae_conditional_same_set` | Choose actions from another set | The dialog refuses | `tst_actions_panel::ae_conditional_same_set` |
| `ae_conditional_move_guard` | Move or delete an action referenced by a conditional | The library refuses; the conditional remains valid | `pictura-script::ae_conditional_move_guard` |
| `ae_conditional_no_document` | Play a conditional with no active document | Playback stops with a named error | `pictura-script::ae_conditional_no_document` |
| `ae_conditional_recursion` | Build a conditional cycle and play | Aborts with a depth error | `pictura-script::ae_conditional_recursion` |
| `ae_history_per_command` | Play two mutating steps | Two history states | `tst_actions_panel::ae_history_per_command` |
| `ae_history_no_group` | Finish a multi-step action | No single action history state | `tst_actions_panel::ae_history_no_group` |

## `document/command-registry`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Menu bar is generated from the table | Open the frame and compare every menu item with the catalogue | Every menu item maps to a catalogue entry | `tst_command_tree::cr_menu_catalogue` |
| Command identifiers are stable | Change a label/shortcut in the Rust table fixture and dispatch by id | The id still resolves | `tst_command_tree::cr_identifiers_stable` |
| `cr_rust_only` | Inspect the tree for a hand-authored table | Only the Rust catalogue and generated header exist | `tst_command_tree::cr_rust_only` |
| `cr_id_drift` | Compare the Rust const-names table and `command_ids.h` | Names and values match | `pictura-script::cr_id_drift` |
| `cr_list_commands` | Compare `describe()`/`list_commands` with the golden snapshot | Identical ids, paths, labels, and states | `tst_command_tree::cr_list_commands` |
| `cr_record_dispatch` | Dispatch a recordable command that reports success while recording | The sink receives id and params | `tst_command_bus::cr_record_dispatch` |
| `cr_record_refused` | Dispatch a recordable command whose handler reports failure | No recording event reaches the sink | `tst_command_bus::cr_record_refused` |
| `cr_replay_executor` | Execute a recorded command with an executor | The executor runs; no dialog | `tst_command_bus::cr_replay_executor` |
| `cr_replay_missing` | Replay a command with no executor | Named refusal with command and step | `tst_command_bus::cr_replay_missing` |
| `cr_control_dispatch` | Dispatch over the control server while recording | The same sink receives the command | `tst_command_bus::cr_control_dispatch` |

## `ui/actions-panel`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ap_exists` | Construct the frame | `actionsPanel` is a `QWidget` in a `PanelGroup` | `tst_actions_panel::ap_exists` |
| `ap_default_hidden` | Start a fresh session | Actions is registered, hidden, reachable from Window > Panels with F9 | `tst_actions_panel::ap_default_hidden` |
| `ap_hosts` | Show, fly out, and float the group | Same tree, toolbar, and menu | `tst_actions_panel::ap_hosts` |
| `ap_flyout_width` | Open the flyout at 300 px | No horizontal scrollbar; all controls visible | `tst_actions_panel::ap_flyout_width` |
| `ap_tree_shape` | Load a set/action/steps | Three rows in check, modal, name columns | `tst_actions_panel::ap_tree_shape` |
| `ap_tree_expansion` | Expand/collapse a set, action, and step; reopen | State is preserved for all three | `tst_actions_panel::ap_tree_expansion` |
| `ap_tree_alt_expand` | Alt-click a set/action triangle | All descendants expand/collapse | `tst_actions_panel::ap_tree_alt_expand` |
| `ap_check_step` | Clear a step check and play | The step is skipped | `tst_actions_panel::ap_check_step` |
| `ap_check_parent` | Click an action/set check | All children follow | `tst_actions_panel::ap_check_parent` |
| `ap_check_alt` | Alt-click a step check | Siblings invert; clicked step unchanged | `tst_actions_panel::ap_check_alt` |
| `ap_check_partial` | Exclude one child | Parent shows partial and red | `tst_actions_panel::ap_check_partial` |
| `ap_modal_toggle` | Toggle an editable modal control | `withDialog` flips; playback follows | `tst_actions_panel::ap_modal_toggle` |
| `ap_modal_required` | Inspect required and none dialog steps | Required is on and disabled; none is off and disabled | `tst_actions_panel::ap_modal_required` |
| `ap_modal_menu_item` | Inspect an Insert Menu Item dialog step | Control is off and disabled; playback still opens the dialog | `tst_actions_panel::ap_modal_menu_item` |
| `ap_modal_parent` | Mix modal and non-modal steps | Parent shows partial red | `tst_actions_panel::ap_modal_parent` |
| `ap_select` | Shift-click and Ctrl/Cmd-click | Standard selection semantics | `tst_actions_panel::ap_select` |
| `ap_rename` | Double-click an action/set name and edit | Name changes and persists | `tst_actions_panel::ap_rename` |
| `ap_overwrite` | Double-click a step, change values, OK | Recorded values replaced | `tst_actions_panel::ap_overwrite` |
| `ap_reorder` | Drag a step, an action, and a set | New order/ownership persists | `tst_actions_panel::ap_reorder` |
| `ap_duplicate` | Alt-drag a set, action, or step | A copy is inserted; source unchanged | `tst_actions_panel::ap_duplicate` |
| `ap_delete_fast` | Alt-click Delete or drag to Delete | Items removed without confirmation | `tst_actions_panel::ap_delete_fast` |
| `ap_button_mode` | Enable Button Mode | Actions become colored buttons in library order; sets/steps hidden | `tst_actions_panel::ap_button_mode` |
| `ap_button_play` | Click an action button | All included steps play | `tst_actions_panel::ap_button_play` |
| `ap_button_restore` | Toggle Button Mode off | Tree state is restored | `tst_actions_panel::ap_button_restore` |
| `ap_menu_order` | Open the panel menu | Rows and separators match the documented order, including Insert Conditional before Insert Path | `tst_actions_panel::ap_menu_order` |
| `ap_menu_enable` | Cycle no selection/action/set/step, then start recording/playback | Rows follow the documented enablement rules, and edit rows lock during record/play | `tst_actions_panel::ap_menu_enable` |
| `ap_menu_checks` | Toggle Button Mode and Allow Tool Recording | Rows are checked accordingly | `tst_actions_panel::ap_menu_checks` |
| `ap_menu_sets` | Put a synthetic `.atn` in the injected presets dir | Its set appears and choosing it loads it | `tst_actions_panel::ap_menu_sets` |
| `ap_menu_close` | Open the menu | Exactly one Close and one Close Tab Group | `tst_actions_panel::ap_menu_close` |
| `ap_toolbar_order` | Inspect the bottom bar | Six buttons in order with tooltips | `tst_actions_panel::ap_toolbar_order` |
| `ap_toolbar_record` | Start/stop recording | Record state and Stop availability update | `tst_actions_panel::ap_toolbar_record` |
| `ap_toolbar_playback` | Start playback | Record/edit disabled; Stop enabled | `tst_actions_panel::ap_toolbar_playback` |
| `ap_dialog_new_action` | Confirm New Action | Action inserted into set; recording starts | `tst_actions_panel::ap_dialog_new_action` |
| `ap_dialog_playback` | Change and confirm Playback Options | Speed/pause applied and persisted | `tst_actions_panel::ap_dialog_playback` |
| `ap_dialog_action_options` | Change an action name, target set, and key | The action moves and persists | `tst_actions_panel::ap_dialog_action_options` |
| `ap_dialog_set_options` | Confirm Set Options | Set name changes and persists | `tst_actions_panel::ap_dialog_set_options` |
| `ap_insert_stop` | Insert Stop and play | Message with Continue appears | `tst_actions_panel::ap_insert_stop` |
| `ap_insert_menu` | Insert Menu Item and play | The command runs; its dialog appears although the modal control is off and disabled | `tst_actions_panel::ap_insert_menu` |
| `ap_insert_conditional` | Both None or an action outside the containing set | Dialog refuses | `tst_actions_panel::ap_insert_conditional` |
| `ap_record_again` | Record Again with a changed modal value | Only that step's values change | `tst_actions_panel::ap_record_again` |
| `ap_allow_tool` | Toggle and record tool strokes | Steps appear only when on | `tst_actions_panel::ap_allow_tool` |
| `ap_record_escape` | Press Escape while recording | Recording stops and earlier steps remain | `tst_actions_panel::ap_record_escape` |
| `ap_play_from_step` | Select a step and Play | Plays from that step to the end | `tst_actions_panel::ap_play_from_step` |
| `ap_play_single` | Ctrl/Cmd-click Play and Ctrl/Cmd-double-click a step | Only that step plays in both cases | `tst_actions_panel::ap_play_single` |
| `ap_play_key` | Press an assigned key | The action plays | `tst_actions_panel::ap_play_key` |
| `ap_play_concurrent` | Start playback and activate Play again | The second start is refused | `tst_actions_panel::ap_play_concurrent` |
| `ap_save_load` | Save a set, reload it | Actions/steps intact | `tst_actions_panel::ap_save_load` |
| `ap_replace` | Replace Actions and confirm | All sets replaced | `tst_actions_panel::ap_replace` |
| `ap_text_dump` | Ctrl+Alt Save and try to load | Load reports not an action set | `tst_actions_panel::ap_text_dump` |
| `ap_state_restore` | Change Button mode, set/action/step expansion, selection, Allow Tool Recording, and Playback Options, then restart | All state restored; missing names are ignored | `tst_actions_panel::ap_state_restore` |
| `ap_library_restore` | Create/load sets and restart | Library restored | `tst_actions_panel::ap_library_restore` |
| `ap_key_assign` | Assign Shift+F5 | Displayed and plays the action | `tst_actions_panel::ap_key_assign` |
| `ap_key_warn` | Assign a command's key | Warning shown; assignment allowed | `tst_actions_panel::ap_key_warn` |
| `ap_key_windows` | Run on Windows or inject the platform branch | F1/Ctrl+F4/Ctrl+F6 refused | `tst_actions_panel::ap_key_windows` |
| `ap_error_parse` | Load a partially valid palette | Parsed sets load; error shown | `tst_actions_panel::ap_error_parse` |
| `ap_error_target` | Play a step with a missing target | Stops and names the step/target | `tst_actions_panel::ap_error_target` |
| `ap_error_empty` | Play an empty action | No command, error, or history | `tst_actions_panel::ap_error_empty` |
| `ap_a11y_state` | Focus rows with a screen reader | Included/partial and modal/partial announced | manual: screen-reader check; model roles are asserted by tst_actions_panel::ap_a11y_model |
| `ap_a11y_key` | Press Space, then Ctrl/Cmd+Space, on a row | Space toggles inclusion; Ctrl/Cmd+Space toggles modal when editable | `tst_actions_panel::ap_a11y_key` |

## `ui/panel-rail`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Every panel exists as a content widget | Show the frame | Each panel is a `QWidget` in `PanelColumn` | `tst_actions_panel::pr_panel_set` |
| Properties shows its empty state | Select nothing | "No Properties" is shown | `tst_properties_panel::plainDocumentShowsNoProperties` |
| Styles takes the former placeholder slot | Fresh session | Styles is grouped with Adjustments | `tst_workspaces::freshSessionUsesEssentialsDefault` |
| `pr_actions_content` | Show the Actions panel | Action tree and toolbar, not a placeholder | `tst_actions_panel::pr_actions_content` |

## Notes

- `ae_play_stepwise` and `ap_a11y_state` are the two manual rows:
  Step By Step needs visual confirmation of the redraw, and a screen-reader
  announcement cannot be asserted offscreen. Automated playback order and the
  model's standard `Qt::CheckStateRole`/accessible roles cover the rest.
- The real-world `.atn` corpus test is optional and self-skips when
  `PICTURA_ATN_CORPUS` is unset. It is named
  `pictura-codec::real_atn_corpus_round_trips_when_present` and is not a spec
  scenario, but it guards the compatibility claim.
