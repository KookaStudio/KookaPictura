# Test Plan

Covers every scenario in the `document/action-step-edits` delta. All rows are
engine tests: the behavior lives in `pictura-script`, and the app adapter only
forwards to the library. Most save/reload tests go through
`ActionLibrary::save_bytes` or `ActionLibrary::set_bytes` and
`ActionLibrary::load_bytes`, then assert on the reloaded step.
`editing_a_parameter_clears_the_imported_descriptor` and
`mapped_steps_reemit_duplicate_unknown_and_unit_items` keep their existing
byte-level `atn`/codec read and write coverage instead.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ase_edit_invalidates` | Load an imported Gaussian Blur step and an imported Insert Path, each carrying an unknown `Xtra` item; change the radius and the path; save and reload | Each written descriptor uses the new payload and no longer carries `Xtra` | `pictura-script::editing_a_parameter_clears_the_imported_descriptor`, `pictura-script::changing_an_imported_insert_path_reauthors_the_step` |
| `ase_noop_keeps_imported` | Load an imported step, call `set_step_params` with its current params, and separately replace an Insert Path carrying unknown content with the same path; save both | Both saves reproduce the imported bytes unchanged and retain `imported()` | `pictura-script::no_op_parameter_edit_keeps_the_imported_form`, `pictura-script::repeated_insert_path_keeps_the_imported_form` |
| `ase_flag_keeps_imported` | Load an imported OPTIONAL Command step (`dialog_options` 0; a MenuItem is forced SPECIFIC and is not modal-editable), toggle enabled and with-dialog, save the set | The saved descriptor equals the imported descriptor | `pictura-script::flag_only_edits_keep_the_imported_form` |
| `ase_structural_keeps_imported` | Load an imported step, move it and duplicate it, save the set | Each saved copy of the step re-emits the imported bytes unchanged | `pictura-script::structural_step_edits_keep_the_imported_form` |
| `ase_rename_play` | Load a set whose imported Play step targets `Target`, rename `Target` to `Renamed`, save and reload | The reloaded Play step targets `Renamed` | `pictura-script::renaming_an_action_updates_an_imported_play_reference` |
| `ase_rename_conditional` | Load a set whose imported Conditional step names `Target` in Then and Else and carries unrelated `jsn` sidecars, rename it, save and reload; separately exercise a decoded sidecar with conflicting duplicate branch keys | Both reloaded branches name `Renamed`; unrelated sidecars remain byte-identical; an unsafe decoded sidecar falls back to re-authoring | `pictura-script::renaming_an_action_updates_imported_conditional_branches`, `pictura-script::renaming_a_conditional_leaves_unrelated_jsn_sidecars_untouched`, `pictura-script::renaming_a_conditional_with_conflicting_duplicate_branches_reauthors` |
| `ase_rename_jsn_only_play` | Load a set whose imported Play step carries the target only in `jsn.action` (no `Nm  `), rename it, save and reload | The reloaded Play step targets `Renamed` | `pictura-script::renaming_a_jsn_only_play_reference_updates_it` |
| `ase_rename_preserves_unknown` | Load an imported Play step with an unknown `Xtra` item, duplicate `Nm  ` and JSON `action` keys, duplicate unknown JSON keys, a high-precision number, noncanonical key order/whitespace, and a non-UTF8 display name; rename its target, save and reload | Every target copy is `Renamed`; `Xtra`, unrelated JSON bytes, and display-name bytes are unchanged | `pictura-script::renaming_keeps_unknown_imported_descriptor_items`, `pictura-script::renaming_a_play_preserves_unrelated_json_bytes` |
| `ase_rename_duplicate_atomic` | Snapshot a library with Play and Conditional callers, rename an action to an existing name, serialize again | The rename is refused; the full library and both byte strings are unchanged | `pictura-script::rejected_duplicate_rename_leaves_bytes_unchanged` |
| `ase_unedited_roundtrip` | Load an imported step with duplicate keys, unknown items, and unit values, then re-encode without editing | The re-encoded bytes equal the source bytes | `pictura-script::mapped_steps_reemit_duplicate_unknown_and_unit_items` |
| `ase_unrelated_unknown_event` | Load a set with a Play step and an unrelated unknown-event step carrying non-UTF8 display-name bytes, rename the Play target, save | The unknown-event raw step before and after the rename is identical | `pictura-script::renaming_an_action_leaves_unrelated_imported_steps_untouched` |

## Verification record

- `cargo fmt --all`: pass.
- Budgeted CMake configure and build: pass. Four stale zero-byte Qt MOC outputs were regenerated. The rebuild after the Clippy type-alias fix also passed.
- `python3 scripts/check-test-plan.py`: pass.
- `openspec validate action-step-edit-ownership --strict`: pass.
- `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`: fmt, Clippy, and build-tree checks passed. The test report recorded 11,811 passed, 152 skipped, and one failure in `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`. The gate stopped before the guard checks.
- Clean `origin/master` GPU reproduction: `tst_present_cache` failed on the same case with the same `samePixels(committed, cpu.image())` assertion.
- Remaining `verify-fast` checks: all passed except `check-milestone-names.py --self-test`.
- Clean `origin/master` milestone-name reproduction: the same `a scoped scan missed a changed violation` assertion failed.
- `openspec validate --all --strict`: 529 specs passed.
- `bash scripts/check-openspec-archived.sh`: pass.
- MSVC string-limit and colour-literal self-tests: pass.

All scenario rows are automated. There are no manual checks.
