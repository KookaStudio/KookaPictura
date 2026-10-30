# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Control mode starts and reports its socket | Launch with `--control`; connect; `status` | Path printed; reply received | manual: a process launch; `scripts/verify-control.sh` runs it in CI |
| No control server without the flag | Create `AgentControl` on an empty store | Not running; no socket | `tst_control_live::offByDefault` |
| A custom socket path is honored | Server on a temp path; connect | Client served | `tst_control_serial::repliesKeepRequestOrder` |
| The socket is user-only | `listen()`; read the socket's mode | No group or other bits | `tst_control_policy::socketIsUserOnly` |
| No eval method exists | Call `eval` restricted and bare | `unknown_method` | `tst_control_policy::noEvalMethodExists` |
| An oversized request is rejected | Write over 1 MiB with no newline | `bad_request` | `tst_control_policy::oversizedRequestIsRejected` |
| `ac_path_refused` | As #646 | `invalid_param` | `tst_untrusted_input::controlRefusesAmbiguousDocumentPaths` |
| `ac_path_save_link` | As #646 | `invalid_param` | `tst_untrusted_input::controlRefusesASaveOverASymlink` |
| `ac_path_save_active_link` | As #646 | `invalid_param` | `tst_untrusted_input::controlRefusesAPathlessSaveThroughAnOpenedSymlink` |
| `ac_path_accepted` | As #646 | Both ok | `tst_untrusted_input::controlAcceptsAPlainAbsolutePath` |
| `ap_off_by_default` | Empty store | Not running | `tst_control_live::offByDefault` |
| `ap_live_start_stop` | Enable; connect; disable | Served, then endpoint and token gone | `tst_control_live::startsAndStopsAtRuntime` |
| `ap_live_settings_persist` | Apply settings; new `AgentControl` | Same settings | `tst_control_live::settingsPersist` |
| `ap_second_window` | Two `AgentControl`s on one endpoint | Second reports; first serves | `tst_control_live::secondWindowDoesNotTakeOver` |
| `ap_sandbox` | Set `FLATPAK_ID`; start | False with a reason | `tst_control_live::sandboxedBuildsDoNotStart` |
| `ap_tier_read_only` | No Edit; `filter`, `status` | `refused`; ok | `tst_control_policy::readOnlyRefusesEdits` |
| `ap_tier_edit_no_save` | Edit only; `save_as` | `refused`; no file | `tst_control_policy::editWithoutSaveRefusesSaving` |
| `ap_every_method_has_tier` | Enumerate the table | Each has a tier; `key` Admin | `tst_control_policy::everyMethodHasATier` |
| `ap_admin_refused` | All grants; `key`; window `pointer` | `refused` | `tst_control_policy::adminMethodsAreRefused` |
| `ap_command_allowlist` | `file.exit`, `select.all`; check each listed id exists | `refused`; ok | `tst_control_policy::commandsAreDefaultDeny` |
| `ap_bare_control` | Unrestricted `key` | Not refused by tier | `tst_control_policy::bareControlIsUnchanged` |
| `ap_path_roots` | Live; open inside a root | Opens; no bar | `tst_control_approval::insideARootDoesNotAsk` |
| `ap_path_outside_headless` | Headless; open outside | `refused` | `tst_control_policy::headlessRefusesOutsideItsRoots` |
| `ap_path_symlink_escape` | Link in root to outside; canonicalize; judge | Ask | `tst_control_policy::symlinkDoesNotEscape` |
| `ap_path_sibling_prefix` | Root `/a/b`, target `/a/b2/x` | Ask | `tst_control_policy::siblingPrefixIsNotInside` |
| `ap_hard_deny` | Allow all on a denied dir | Refuse | `tst_control_policy::protectedDirectoriesAreRefused` |
| `ap_approval_matrix` | Each mode × inside/outside/overwrite/known | Matches the table | `tst_control_policy::approvalModesDecide` |
| `ap_prompt_parks` | Ask target, then `status` | Bar shows; no replies | `tst_control_approval::requestWaitsForApproval` |
| `ap_prompt_allow` | Allow | Replies in order | `tst_control_approval::allowRunsTheRequest` |
| `ap_prompt_deny` | Deny | `refused`; no write | `tst_control_approval::denyRefusesTheRequest` |
| `ap_prompt_timeout` | Short timeout; wait | `approval_timeout`; bar hidden | `tst_control_approval::unansweredPromptTimesOut` |
| `ap_prompt_disconnect` | Disconnect while waiting | Bar hidden; next client served | `tst_control_approval::disconnectWithdrawsThePrompt` |
| `ap_live_dirty_close` | Dirty document; `close` | `unsaved`; still open | `tst_control_live::dirtyCloseIsRefused` |
| `ap_human_close_prompts` | Scope resolves Cancel; ask outside | Normal policy used | `tst_control_live::personsPromptIsUnaffected` |
| `ap_human_busy` | Open dialog; `filter`; `status` | `human_busy`; ok | `tst_control_live::editsWaitForAPerson` |
| `ap_open_deferred` | Dialog open; socket `open` | Opens; dialog visible | `tst_control_live::openLeavesAPersonsDialogAlone` |
