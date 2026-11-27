# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `gm_unlisted` | In the drift self-test's temporary tree, add `check-planted.py` without a row and validate. | Validation fails and names the script. | `check-gate-manifest::unlisted_script_is_rejected` |
| `gm_new_check` | Add a synthetic guards row and compare local-full and CI selections. | Both selections contain the new ID with no runner changes. | `check-gate-manifest::new_entry_selects_both_runners` |
| `gm_mandatory_ci` | Remove each mandatory ID from a synthetic manifest in turn. | Validation fails and names the missing ID. | `check-gate-manifest::mandatory_ci_cannot_shrink` |
| `gm_no_local_only` | Schedule a synthetic local row with neither CI placement. | Validation rejects the unscoped local-only row. | `check-gate-manifest::local_only_placement_is_rejected` |
| `gm_full_opt_in` | Select a deferred expensive row with the full-local flag absent, then present. | Fast omits it with a deferral notice; full and CI include it. | `check-gate-manifest::full_local_requires_explicit_flag` |
| `gm_runner_parity` | Mark a synthetic row CI-only and validate. | Validation fails because local full lacks it. | `check-gate-manifest::ci_entry_requires_local_full` |
| Docs-only change | Select `docs-only` from the real manifest. | Build commands are absent and all `ci_guards` rows remain available to CI. | `check-gate-manifest::docs_only_selection_skips_builds` |
| Code change | Select local fast and local full from the real manifest. | Fast contains the existing edit-loop checks; full is a superset. | `check-gate-manifest::local_full_contains_fast` |
| Guards fail | Run two synthetic serial commands, the second returning non-zero. | Execution stops at the second ID and reports its captured output. | `check-gate-manifest::serial_runner_stops_on_failure` |
| The fast full gate runs the worktree-hook check | Inspect local-fast and local-full selections. | Both contain `worktree-hook`. | `check-gate-manifest::mandatory_local_checks_remain` |
| The guards workflow runs the archive check | Inspect the guards CI selection. | It contains `archive`. | `check-gate-manifest::mandatory_ci_cannot_shrink` |
| `vh_docs_only_guard` | Inspect the docs-only selection. | It contains `selftest-codes` and no build command. | `check-gate-manifest::docs_only_selection_skips_builds` |
| `vh_parallel_checks` | Run passing synthetic parallel commands and collect results. | Every result is captured in manifest order and execution succeeds. | `check-gate-manifest::parallel_runner_collects_all_results` |
| `vh_parallel_failure` | Run passing and failing synthetic parallel commands. | All commands finish and the failing output is retained. | `check-gate-manifest::parallel_runner_collects_all_results` |
| `gm_ci_failure` | Run a failing synthetic serial command. | Its manifest ID and output identify the failure. | `check-gate-manifest::serial_runner_stops_on_failure` |
| Docs-only working diff | Inspect the real docs-only selection. | Guard, archive, test plan, self-test codes, and OpenSpec are present; builds are absent. | `check-gate-manifest::docs_only_selection_skips_builds` |
| Clean tree or code change | Compare real local-fast and local-full selections. | The appropriate non-docs placement is selected and full is a superset. | `check-gate-manifest::local_full_contains_fast` |
| `gm_build_lock_ci` | Inspect guards CI and local-full selections. | Both contain `build-lock`. | `check-gate-manifest::mandatory_ci_cannot_shrink` |
