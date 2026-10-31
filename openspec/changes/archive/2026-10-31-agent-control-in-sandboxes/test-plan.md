# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ap_off_by_default` | Start with an empty session store | No endpoint exists | `tst_control_live::offByDefault` |
| `ap_live_start_stop` | Enable, connect, disable | Client served; endpoint and token gone | `tst_control_live::startsAndStopsAtRuntime` |
| `ap_live_settings_persist` | Change settings, load a new `AgentControl` | Same settings | `tst_control_live::settingsPersist` |
| `ap_second_window` | Start a second `AgentControl` while one serves | Conflict reported; first keeps serving | `tst_control_live::secondWindowDoesNotTakeOver` |
| `ap_dead_holder` | Leave a lock file naming a running PID, with no holder; enable | Server starts | `tst_control_live::deadHolderDoesNotBlock` |
| `ap_sandbox` | Enable with `FLATPAK_ID`, then with `SNAP`, set | Server runs both times | `tst_control_live::sandboxedBuildsStart` |
| `ap_sandbox_endpoint` | Resolve the endpoint with `FLATPAK_ID` set | Under `$XDG_RUNTIME_DIR/app/$FLATPAK_ID` | `tst_control_live::flatpakInstancesShareTheEndpoint` |
| `ap_endpoint_outside_flatpak` | Resolve the endpoint with `SNAP` set and with neither | Under `$XDG_RUNTIME_DIR/pictura` | `tst_control_live::otherBuildsKeepTheirEndpoint` |
| `ap_sandbox_round_trip` | Install the package, serve from the real window, run the advertised command | MCP exchange completes; stdout is only protocol | manual: needs an installed package in a real sandbox, so `scripts/check-sandbox-mcp.sh` runs it in the Flatpak and Snap CI jobs, not in Qt Test |
| `apf_enable` | Tick Enable, then untick | Server runs, then stops; status follows | `tst_preferences_agent::enablingStartsTheServer` |
| `apf_permissions` | Tick Edit and Save, pick Manual | Running policy and a new page show them | `tst_preferences_agent::permissionsApplyAtOnce` |
| `apf_save_needs_edit` | Untick Edit | Save unticked and disabled | `tst_preferences_agent::saveNeedsEdit` |
| `apf_allow_all` | Pick Allow all, then another mode | Warning shows, then hides | `tst_preferences_agent::allowAllWarns` |
| `apf_folders` | Add folders, remove one | Remaining folder stored | `tst_preferences_agent::foldersPersist` |
| `apf_snippet` | Read the snippet in plain, AppImage, Flatpak and Snap environments | Command and args match each build | `tst_preferences_agent::clientSnippetIsValid` |
| `apf_limits` | Show the page | Says a shell is outside the permissions | `tst_preferences_agent::limitsAreStated` |
| `apf_sandbox` | Show the page under Flatpak and under Snap | Controls enabled, no registration box, note shown | `tst_preferences_agent::sandboxedBuildsExplainThemselves` |
| `apf_stop_pending` | Stop the server while a request waits | Bar hides | `tst_preferences_agent::stoppingWithdrawsAnApproval` |
| `apf_menu` | Invoke Edit > Preferences > Agent Control | Dialog opens on the page | `tst_preferences_agent::menuOpensThePage` |
| Flatpak host folders | Run `flatpak override --user --filesystem=~/Pictures <id>`, restart, add the folder in Preferences, ask an agent to open a file there by host path | The file opens, and the page lists the real path | manual: needs a desktop portal, and the CI runner has none (#647) |
| Real client attach | Paste the snippet into Claude Code on a desktop, with the Flatpak and with the Snap | The client lists the tools and reads the window's status | manual: needs a real MCP client and desktop session |
