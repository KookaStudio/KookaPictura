# Test Plan — edition-gating

One row per scenario in `specs/ui/edition-gating/spec.md`. Coverage names an
automated test (`<crate>::<fn>` for Rust, `tst_<suite>::<case>` for Qt Test) or
`manual: <reason>`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Standard is selected | Parse `PICTURA_EDITION=standard`; build a frame with it set | `Edition::Standard`; the Standard surface is active | `pictura_core::edition::tests::standard_and_extended_parse_case_insensitively`; `tst_edition_gating::editionGating` |
| Extended is the default | Parse an unknown value; build a frame with the variable unset | `Edition::Extended`; the Extended surface is active | `pictura_core::edition::tests::unknown_value_defaults_to_extended`; `tst_edition_gating::editionGating` |
| Count is absent in Standard | Set `PICTURA_EDITION=standard`, open the `I` group flyout, try `setActiveTool(Count)` | No Count action; the active tool does not become Count | `tst_edition_gating::editionGating` |
| Count is present in Extended | Open the default frame's `I` group flyout | Count is listed and `toolAvailable(Count)` is true | `tst_edition_gating::editionGating` |
| Extended-only commands are absent in Standard | Set `PICTURA_EDITION=standard`, query the 3D, Measurement Log and Count overlay ids | `registry->action(id)` is null for each | `tst_edition_gating::editionGating` |
| Timeline stays in Standard | Set `PICTURA_EDITION=standard`, query `window.panels.timeline` | The action is registered | `tst_edition_gating::editionGating` |
| Extended keeps the commands | Query the default frame's 3D, Measurement Log and Count overlay ids | Each action is registered | `tst_edition_gating::editionGating` |

## Supporting tests

- `pictura_script::tool_catalogue::tests::extended_only_tools_are_count_and_the_3d_rig`
  and `pictura_script::command_catalogue::tests::extended_only_command_ids` pin
  the catalogue policy behind the gate.
- `pictura_core::edition::tests::timeline_is_all_editions` pins the CS6
  correction that video is not Extended-only.
