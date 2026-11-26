# Design

## Context

See `proposal.md` for the problem and scope. Fetched `origin/master` declares
270 suite sources. Eleven suites currently use four CTest shares and
`tst_ui_scale` uses eight, so the current registry produces 310 entries. The two
long suites still use one entry each.

Every suite currently owns its `QTEST_MAIN` or hand-written `main`. Most include
their own generated `tst_<name>.moc`. CMake's global `AUTOMOC` therefore creates
an empty `mocs_compilation.cpp` for each executable. The shared precompiled
header is reused across all suite targets, and CMake requires matching compiler
definitions for that reuse.

## Goals / Non-Goals

**Goals:**

- Reduce executable and AUTOMOC ownership from one per source to one per subsystem.
- Keep suite selection explicit and keep one top-level process per CTest entry.
- Preserve test bodies, assertions, JUnit names, shard selection, timeouts, and failure returns.
- Add the two long suites to the existing shard mechanism without another scheduler.

**Non-Goals:**

- Change application behavior, the self-test, Rust tests, or test assertions.
- Add dependencies, a plugin loader, a generated test registry format, or a new CTest layer.
- Change application behavior, the self-test, Rust tests, or test assertions.

## Decisions

### 1. One CMake registry owns suites and driver membership

`PICTURA_QT_TESTS` becomes a derived flat list. Fourteen explicit subsystem
lists own the actual registry. CMake derives both the flat suite list and a
suite-to-driver map from those lists, then builds one
`tst_driver_<subsystem>` executable per list.

A configure-time guard compares the derived list with the standalone suite
sources in the test directory. The glob is used only for that comparison, never
as a source list. `tst_actions_panel_cases.cpp` is explicitly listed as an
included implementation source because `tst_actions_panel.cpp` includes it.
Configuration fails on a missing source, an unregistered standalone source, or a
suite that appears in two subsystem lists. This keeps the explicit registry
while making silent suite loss impossible.

The membership is:

| Driver | Suites |
|---|---|
| shell | `tst_smoke`, `tst_structured_logging`, `tst_history_log`, `tst_file_info`, `tst_file_menu`, `tst_gpu_backend`, `tst_offthread_jobs`, `tst_panel_host`, `tst_menu_registrar`, `tst_command_tree`, `tst_edition_gating`, `tst_toolbar_shape`, `tst_command_bus`, `tst_merge_commands`, `tst_system_info`, `tst_tool_hint_bar`, `tst_icon_assets`, `tst_selftest_report`, `tst_dialog_chrome`, `tst_keyboard_shortcuts`, `tst_search_bar`, `tst_native_menu_bar`, `tst_untrusted_input`, `tst_theme_apply`, `tst_test_support`, `tst_app_updater`, `tst_notification_status_bar` |
| document | `tst_document_version_policy`, `tst_multi_file_open`, `tst_document_api`, `tst_document_registry`, `tst_document_session`, `tst_session_ports`, `tst_edit_clipboard`, `tst_history_snapshots`, `tst_edit_refresh`, `tst_image_ops`, `tst_image_rotate`, `tst_canvas_pan`, `tst_initial_view`, `tst_new_document`, `tst_image_mode`, `tst_hdr_conversion`, `tst_align_distribute`, `tst_measurement_scale`, `tst_memory_governor`, `tst_busy_job`, `tst_document_jobs`, `tst_edit_shortcuts`, `tst_workspaces`, `tst_floating_workspaces`, `tst_floating_workspaces_restart`, `tst_zoom_tool`, `tst_large_document`, `tst_tool_commit`, `tst_recovery`, `tst_session_store`, `tst_settings_backup`, `tst_rulers_guides`, `tst_view_menu`, `tst_panels_parity`, `tst_variables`, `tst_screen_modes`, `tst_loading_tab`, `tst_document_windows`, `tst_document_float`, `tst_document_arrange`, `tst_document_navigate`, `tst_canvas_context_menus` |
| io | `tst_open_save`, `tst_place_fit`, `tst_place_sequence`, `tst_import_save_coordinator`, `tst_save_for_web`, `tst_export_as`, `tst_export_batch`, `tst_export_layers`, `tst_file_export_tools`, `tst_load_files_into_stack`, `tst_statistics`, `tst_webp`, `tst_save_options`, `tst_psd_compatibility`, `tst_save_format_options`, `tst_save_format_catalog`, `tst_batch_dialog`, `tst_batch_run`, `tst_save_format_dialogs`, `tst_save_gif_copy`, `tst_print`, `tst_save_pdf`, `tst_import_pdf`, `tst_image_import` |
| selection | `tst_selection_api`, `tst_mask_edit`, `tst_quick_mask`, `tst_quick_mask_regressions`, `tst_canvas_mask_domain`, `tst_crop_straighten`, `tst_crop_shield`, `tst_crop_settings`, `tst_crop_grip`, `tst_crop_snap`, `tst_crop_session`, `tst_crop_options`, `tst_crop_preview`, `tst_crop_cursor`, `tst_perspective_crop`, `tst_color_range`, `tst_refine_edge`, `tst_replace_color`, `tst_selection_aa`, `tst_selection_alt`, `tst_selection_move`, `tst_selection_preview`, `tst_selection_save_load` |
| paint | `tst_paint_api`, `tst_color_replacement`, `tst_options_bars_parity`, `tst_fill_tools`, `tst_eraser_tools`, `tst_retouch_tools`, `tst_stamp_hold`, `tst_brush_airbrush`, `tst_brush_dynamics`, `tst_bristle_brushes`, `tst_pen_tools`, `tst_path_selection_tools`, `tst_shape_tools`, `tst_shape_transform`, `tst_rotate_view`, `tst_move_feedback`, `tst_smart_guides`, `tst_tool_options_persistence`, `tst_brush_tip_preview`, `tst_brush_outline` |
| transform | `tst_transform_api`, `tst_transform_options`, `tst_transform_controls`, `tst_transform_skew`, `tst_transform_distort`, `tst_transform_perspective`, `tst_transform_lock`, `tst_transform_overlay`, `tst_transform_warp`, `tst_transform_commands`, `tst_transform_selection` |
| layers | `tst_layers_mask_commands`, `tst_mask_link_state`, `tst_last_layer`, `tst_layers_panel`, `tst_layer_target`, `tst_linked_layers`, `tst_layers_panel_fixes`, `tst_layers_panel_bundle`, `tst_layers_row_menu`, `tst_layers_row_model`, `tst_layers_preview_coalescing`, `tst_layer_content`, `tst_layers_snapshot`, `tst_canvas_backdrop`, `tst_present_cache`, `tst_damage_bounds`, `tst_fill_layer_editor`, `tst_layer_commands`, `tst_properties_panel`, `tst_properties_preview`, `tst_channels_panel`, `tst_channel_options`, `tst_paths_panel`, `tst_layer_style`, `tst_blend_mode_list`, `tst_auto_align`, `tst_auto_blend`, `tst_layers_smart_filters`, `tst_smart_filter_apply`, `tst_layers_effects`, `tst_smart_object_actions`, `tst_vector_smart_object`, `tst_vector_paste`, `tst_smart_object_relink`, `tst_layer_comps_panel`, `tst_layer_comps_export`, `tst_nested_layer_target` |
| type | `tst_fonts`, `tst_type_tools`, `tst_type_panels`, `tst_type_color_preview`, `tst_type_edit_damage`, `tst_paragraph_type`, `tst_type_on_path`, `tst_character_paragraph_panels`, `tst_type_session_panels`, `tst_paragraph_styles_panel`, `tst_character_styles_panel`, `tst_type_preferences`, `tst_warp_text`, `tst_type_menu`, `tst_find_replace`, `tst_type_placeholder`, `tst_text_gamma_fonts` |
| ui | `tst_design_system`, `tst_design_system_dialog`, `tst_ui_scale`, `tst_ui_localization`, `tst_shared_widgets`, `tst_value_input_enter`, `tst_info_panel`, `tst_histogram_panel`, `tst_swatches_panel`, `tst_preset_manager`, `tst_color_picker`, `tst_eyedropper`, `tst_color_settings`, `tst_proof_setup`, `tst_profile_dialog`, `tst_profile_warnings`, `tst_adjustment_presets`, `tst_adjustments_panel`, `tst_actions_panel` |
| filters | `tst_filter_menu`, `tst_filter_gallery`, `tst_fade`, `tst_lens_flare_dialog`, `tst_picture_frame_dialog`, `tst_reduce_noise_dialog`, `tst_lighting_effects_dialog`, `tst_shear_dialog`, `tst_displace`, `tst_de_interlace`, `tst_custom_filter`, `tst_maximum_filter`, `tst_digimarc_watermark`, `tst_shape_blur`, `tst_smart_blur`, `tst_lens_blur_dialog`, `tst_blur_gallery_dialog`, `tst_tilt_shift`, `tst_blur_gallery_effects`, `tst_path_blur`, `tst_spin_blur`, `tst_liquify_dialog`, `tst_adaptive_wide_angle`, `tst_lens_correction`, `tst_lens_profiles`, `tst_vanishing_point`, `tst_vanishing_point_retouch`, `tst_vanishing_point_files` |
| imaging | `tst_apply_image`, `tst_calculations`, `tst_match_color`, `tst_image_adjustments` |
| control | `tst_client_registration`, `tst_control_approval`, `tst_control_editing`, `tst_control_limits`, `tst_control_live`, `tst_control_metrics`, `tst_control_metrics_off`, `tst_control_modal_guard`, `tst_control_policy`, `tst_control_serial` |
| preferences | `tst_preferences_agent`, `tst_preferences_store`, `tst_preferences_reset`, `tst_update_preferences` |
| raw | `tst_pictura_raw`, `tst_acr_settings`, `tst_acr_dialog`, `tst_raw_open` |

The alternative, keeping the flat list and adding a separate driver assignment
list, creates two places that must agree. It is rejected because silent suite
loss is the failure this change must avoid.

### 2. Suite entry points register themselves without changing test bodies

`qt_test_support.h` gains entry macros for `QTEST_MAIN`, `QTEST_GUILESS_MAIN`,
and the sharded path. The ordinary macros place the existing Qt macro expansion
in a derived namespace named `picturaQtTest_<TestClass>` and register that
function. The derived prefix keeps the namespace distinct from the global
class. The sharded macro does the same around a small `main` that calls the
existing `shardedMain<TestClass>`. The generated moc include stays outside the
namespace.

The 17 hand-written mains keep their behavior. Sixteen gain the same derived
namespace and one registration call. `tst_test_support` uses the sharded entry
macro because its hand-written body only called `shardedMain`:

- The 12 currently sharded suites: `tst_canvas_mask_domain`,
  `tst_document_arrange`, `tst_document_float`, `tst_document_windows`,
  `tst_edit_refresh`, `tst_floating_workspaces`, `tst_import_pdf`,
  `tst_loading_tab`, `tst_print`, `tst_save_pdf`, `tst_ui_scale`, and
  `tst_workspaces`.
- `tst_raw_open`, `tst_theme_apply`, and `tst_test_support`, which already call
  `shardedMain` without being registered in shards.
- `tst_structured_logging` and `tst_preferences_reset`, which keep their custom
  child-mode handling.

`tst_quick_mask` and `tst_document_navigate` are the only suites that switch
from `QTEST_MAIN` to the sharded entry macro. Their reset bodies stay intact.
Test class names are unique across the current registry, and the derived
namespace prevents a class and namespace name collision. The installed Qt 6
`QTEST_MAIN` expands to a function with function-local application setup; it
does not add the Qt 5 GPU-blacklist globals that would require global scope.

A source-specific `-Dmain=...` would avoid editing suite files, but CMake's
`target_precompile_headers(... REUSE_FROM ...)` requires matching compiler
definitions. That route risks invalidating the shared PCH for every suite. A
generated wrapper that includes each `.cpp` would also hide the `.moc` includes
from normal AUTOMOC source ownership. Explicit entry wrappers are the smaller
reliable change.

### 3. The shared support library owns dispatch

The registry stores a suite name and function pointer. A function-local static
container behind an accessor creates the registry on first registration, so
cross-translation-unit static initialization order cannot destroy it. A
registered suite name comes from a source path's basename without `.cpp`, with
both slash styles accepted so the same code works on Windows. Tests may pass an
explicit fake source path to register a probe without taking the real suite's
name.

`runSuiteDriver` requires at least three arguments, `argv[1]` equal to
`--suite`, and a non-empty `argv[2]`. It accepts only an exact registered name.
`--suite=name`, a `.cpp` suffix, a selector in another position, and an unknown
name fail. It requires exactly one matching registration. Two registrations
with the same name fail before either starts.

The dispatcher rebuilds the argument list with the suite name as `argv[0]` and
every argument from `argv[3]` onward unchanged. That preserves the application
name and settings paths that Qt derives from the old per-suite executable name.
It does not create a `QApplication`, run a loop, or start child processes. The
selected suite's existing main still owns application setup and QTest
execution. A suite failure returns its original value.

The alternative, a generated table of declarations per driver, would require
CMake to parse test class names or a second handwritten table.
Self-registration uses the entry macro's existing class token and needs no
generated C++.

### 4. CTest names the suite, not the driver

Each `add_test` keeps its current name and JUnit filename. Its command becomes
`<driver> --suite <name> -o <entry>.xml,junitxml`. Sharded entries keep their
current `<suite>_<index>` names and `PICTURA_TEST_SHARD=<index>/<count>`
environment. Timeouts stay on the test properties, not in the driver.

Quick Mask moves from one entry to four. Its existing 300-second timeout is
applied to all four. Document Navigate uses the default 120 seconds on each of
four shares. Both suite mains call `shardedMain`, so the existing declaration
order split and named-function behavior remain the only shard algorithm.

Registration accounting is explicit. Consolidation alone preserves all 310
current entries. The requested sharding removes the two unsharded names and
adds eight share names, producing 316 entries. A literal requirement to keep
both total count and names unchanged cannot coexist with adding those shares.
The proposal calls out this necessary delta rather than hiding it.

### 5. Grouped AUTOMOC keeps real meta-objects and drops empty stubs

Each driver target owns all of its suite `.cpp` files. Every suite still
includes its own `tst_<name>.moc`, so CMake generates the real meta-object and
the suite translation unit includes it. Files not included by a suite would
land in the driver's `mocs_compilation.cpp`; the current ownership rule means
that unit has no moc output. AUTOMOC ownership therefore drops from one target
per suite to one per driver.

`pictura_test_support` and `pictura_test_pch` keep `AUTOMOC OFF`. The PCH target
and `REUSE_FROM` wiring stay unchanged. No source-specific compiler definition
is added to work around main symbols.

### 6. Per-suite build exceptions follow the owning driver

Five suites have special target settings:

- `tst_structured_logging`, `tst_ui_localization`, and `tst_preferences_store`
  keep their executable-path compile definitions, applied only to their source.
  Those three sources set `SKIP_PRECOMPILE_HEADERS` so a source-only definition
  cannot invalidate the reused PCH.
- Their drivers, and the driver containing `tst_update_preferences`, keep the
  dependency on `pictura`.
- `tst_selftest_report` keeps `selftest_report.cpp` and `selftest_run.cpp` in its
  driver target.

Tests that restart `QCoreApplication::applicationFilePath()` must pass
`--suite <their-suite>` before child-mode flags or named test functions. This
applies to structured logging, preferences reset, recovery, and control metrics
off. All four call sites are test code and all four suites are unsharded. The
child receives the same arguments after dispatch removes the suite selector.

### 7. Dispatch gets a runnable check without another CTest entry

`tst_test_support` registers a probe under an explicit fake source path and
calls `runSuiteDriver` directly. It checks argument forwarding, suite-name
`argv[0]`, return-code propagation, path normalization, malformed selectors,
unknown names, and duplicate registrations. A second probe name is used for
the duplicate case so the normal probe remains selectable. This keeps the
CTest inventory unchanged while exercising the new branch logic.

## Risks / Trade-offs

- Grouping suites runs every suite's static initializers in the driver process. The current suite sources contain no `Q_COREAPP_STARTUP_FUNCTION`, `Q_IMPORT_PLUGIN`, or `Q_GLOBAL_STATIC`. The implementation task still audits global helpers, duplicate class names, and static objects in each proposed source set. Colliding helpers move into anonymous namespaces.
- Duplicate registrations can collide at startup. Mitigation: derived class namespaces and dispatch-time duplicate rejection before either suite starts. The probe tests this without terminating the process.
- Driver binaries grow because each contains several suites. Mitigation: 14 subsystem groups instead of one monolithic binary; link and cold/warm costs remain a required follow-up measurement.
- Self-spawn call sites could forget `--suite`. Mitigation: the four known call sites are updated and covered by their existing tests.
- The driver executable path differs from the old suite path. Mitigation: dispatch supplies the suite name as `argv[0]`, preserving Qt's application name and settings-path derivation. `QCoreApplication::applicationFilePath()` still returns the driver, so self-spawn call sites carry the selector.
- The issue's unchanged-count wording conflicts with the requested sharding. Mitigation: preserve every unrelated entry and record the exact six-entry increase.
- `check-build-tree.sh` uses CTest's `command[0]` and the entry name to find the suite source, so driver commands remain compatible. Driver targets keep the `tst_driver_<subsystem>` prefix, so `check-local-build-caching.sh` still finds their object files with its existing `tst_*` pattern. `test-report.sh` consumes CTest names and JUnit paths. No script or CI command directly invokes a per-suite binary.
- The measurements in `tasks.md` and `test-plan.md` come from a clean `origin/master` worktree and this branch. Six Qt Test failures and one non-reproducing SEGFAULT in `tst_client_registration` are pre-existing on `origin/master`.

## Migration Plan

1. Add the registry and dispatch checks, then route every suite through its entry wrapper.
2. Build the subsystem registry, add its completeness guard, and move per-suite CMake exceptions to their owners.
3. Add the two long suites to the shard list and apply Quick Mask's timeout to every share.
4. Review the complete diff for registry membership, global-helper isolation, CTest names, JUnit paths, timeouts, and unchanged assertions.
5. Configure and build once, compare `ctest --show-only=json-v1` output with `origin/master`, run the Qt Test layer, inspect generated AUTOMOC units, and record cold and warm before/after numbers.
6. Roll back by reverting the change as one unit. No persisted data or application migration exists.

## Open Questions

None; the measurements are recorded in `tasks.md` and `test-plan.md`.
