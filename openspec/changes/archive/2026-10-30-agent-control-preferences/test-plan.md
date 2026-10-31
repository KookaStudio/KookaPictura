# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `apf_enable` | Tick, untick | Runs, stops; status text | `tst_preferences_agent::enablingStartsTheServer` |
| `apf_permissions` | Tick Edit, Save; pick Manual | Policy updated; new page shows it | `tst_preferences_agent::permissionsApplyAtOnce` |
| `apf_save_needs_edit` | Untick Edit | Save off and disabled | `tst_preferences_agent::saveNeedsEdit` |
| `apf_allow_all` | Pick Allow all; pick Manual | Warning shows; hides | `tst_preferences_agent::allowAllWarns` |
| `apf_folders` | Add two, remove one | One remains | `tst_preferences_agent::foldersPersist` |
| `apf_snippet` | Parse the snippet | Command and `--mcp` | `tst_preferences_agent::clientSnippetIsValid` |
| `apf_limits` | Read the note | Mentions a shell | `tst_preferences_agent::limitsAreStated` |
| `apf_sandbox` | Set `FLATPAK_ID` | Controls disabled; reason shown | `tst_preferences_agent::sandboxedBuildsExplainThemselves` |
| `apf_stop_pending` | Park a request; untick Enable | Bar hidden | `tst_preferences_agent::stoppingWithdrawsAnApproval` |
| `apf_menu` | Dispatch the command | Dialog on the page | `tst_preferences_agent::menuOpensThePage` |
