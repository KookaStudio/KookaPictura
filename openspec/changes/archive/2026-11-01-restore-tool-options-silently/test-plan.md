# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `top_round_trip` | Change several groups, save the session, open a new window | Every changed value is restored; sibling tools keep their defaults | `tst_tool_options_persistence::optionsSurviveARestart` |
| `top_bad_entries` | Restore an out-of-range type size, a non-object group, and an unknown key | The size is clamped; the other groups keep defaults | `tst_tool_options_persistence::badEntriesKeepDefaults` |
| `top_silent_restore` | Attach listeners to the brush, selection, and crop groups, then apply a stored object | Values are restored and no listener fires | `tst_tool_options_persistence::restoreDoesNotNotify` |
| `top_missing_default` | Apply a group that omits fields it held before | Each omitted field is its default, not the previous value | `tst_tool_options_persistence::missingFieldsRestoreDefaults` |
| `ctl_tool_options` | Patch brush size through the control server, read back, send a non-object | Size 321 is reported and used; readback works; non-object is `invalid_param` | `tst_tool_commit::toolOptionsPatchAndReadBack` |
| `ctl_tool_options_merge` | Patch one brush field on a controller with non-default siblings | The patch changes one field, keeps the others, and notifies | `tst_tool_options_persistence::patchMergesAndNotifies` |
