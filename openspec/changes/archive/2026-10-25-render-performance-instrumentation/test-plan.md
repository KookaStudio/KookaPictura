# Test plan

The Rust tests run under nextest. The Qt suites run offscreen through CTest.
`tst_control_metrics` sets `PICTURA_METRICS=1` in `initTestCase`, before
anything reads the level. `tst_control_metrics_off` leaves it unset, because the
level is read once per process.

## verification/performance-metrics

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Off by default | Build the metrics state with level 0; open and close an edit record and record a phase. | No line is formatted, and no record or total is kept. | `pictura-app::metrics_off_retains_nothing` |
| Metrics do not change output | Apply the same New Layer + opacity edit sequence to two views, one with metrics state on and one off. | The composites are byte-identical. | `pictura-app::metrics_on_and_off_composite_identically` |
| The stroke report survives | Start a stroke session, record phases, format the report. | The existing labels appear in the report. | `pictura-app::paint_timing_report_keeps_phase_labels` |
| A full recomposite is counted | With metrics state on, apply `Edit::Full`. | The `recomposite_full_composite` count goes up by 1. | `pictura-app::full_recomposite_is_counted` |
| A Qt-side phase is recorded | Call `record_metric_phase("layers_panel_refresh", 1500)` with metrics on and read the totals. | The phase has count 1 and 1.5 ms. | `pictura-app::bridge_phase_is_recorded` |
| One record per committed edit | With metrics on, run a New Layer edit through the transaction. | Exactly one record, labelled `New Layer`. | `pictura-app::new_layer_produces_one_edit_record` |
| An edit with no composite says so | With metrics on, commit `Edit::Inserted`. | `composited=false`, and no composite phase in the record. | `pictura-app::compositeless_edit_reports_no_composite` |
| Canvas bytes for a full composite | Run a CPU `composite_rgba` on a W×H document and read the largest-canvas counter. | At least W×H×4. | `pictura-render::largest_canvas_covers_full_document` |
| A decline is attributed | Call `composite_gpu` on a Dissolve stack (no device needed). | The unsupported-mode decline count increases. | `pictura-render::dissolve_decline_is_counted` |
| No timestamps, no fake zero | Format an edit record whose device time is `None`. | The line has no `gpu_ms=` field. | `pictura-app::record_without_device_time_omits_field` |
| An edit record line | Format an edit record with metrics on. | It starts with `pictura metrics: edit label=`. | `pictura-app::edit_record_line_format` |

## document/edit-transaction

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `et_bounded` | Existing coverage. | Unchanged. | `pictura-app::a_bounded_outcome_requests_one_region`, `pictura-app::layer_opacity_region_refresh_equals_a_full_recomposite` |
| `et_paint_regions` | Existing coverage. | Unchanged. | `pictura-app::a_multi_rect_outcome_requests_one_blit`, `pictura-app::a_multi_rect_stroke_commit_equals_a_full_recomposite` |
| `et_meta` | Existing coverage. | Unchanged. | `pictura-app::a_metadata_outcome_notifies_without_compositing` |
| `et_record` | Existing coverage. | Unchanged. | `pictura-app::an_already_presented_edit_records_without_notifying` |
| `et_structural` | Existing coverage. | Unchanged. | `pictura-app::a_structural_outcome_clears_links_and_rebuilds` |
| `et_unbounded` | Existing coverage. | Unchanged. | `pictura-app::a_full_outcome_rebuilds_once` |
| `et_inserted` | Link two layers, then apply `Edit::Inserted`. | The links are empty, history goes up by 1, the effect is Notify, and the composite and `frame_revision` are unchanged. | `pictura-app::an_inserted_outcome_drops_links_without_compositing` |

## compositing/described-damage

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| New Layer runs no composite | In a CPU view, call `add_layer` above an unclipped layer; compare the stored composite with `composite_rgba`. Then, in the Qt test, read `frame_revision` before and after `add_layer_in`. | No composite ran and the bytes are equal; `frame_revision` is unchanged. | `pictura-app::new_layer_keeps_composite_exact`, `tst_edit_refresh::newLayerDoesNotRecomposite` |
| New Group runs no composite | Same steps with `add_group` / `add_group_in`. | Same. | `pictura-app::new_group_keeps_composite_exact`, `tst_edit_refresh::newGroupDoesNotRecomposite` |
| Insertion under a clipped sibling recomposites | Run the predicate on an unclipped layer inserted below a clipped one. | false. | `pictura-render::insertion_under_clipped_sibling_is_not_neutral` |
| A first clipped layer recomposites | Run the predicate on a clipped layer above a base that has no run. | false. | `pictura-render::first_clipped_layer_is_not_neutral` |
| Joining an existing clipped run is neutral | Run the predicate on a clipped layer above a clipped layer. | true. | `pictura-render::joining_a_clipped_run_is_neutral` |
| The first layer of an empty document recomposites | Run the predicate after adding to an empty document. | false. | `pictura-render::first_layer_of_empty_document_is_not_neutral` |
| Mode-neutral fill recomposites | Create through `add_layer_full` with Overlay and neutral fill. | false. | `pictura-render::mode_neutral_fill_is_not_neutral` |

A property test also backs the predicate:
`pictura-render::neutral_insertions_preserve_composite` inserts at every position
of clipped, grouped, pass-through, knockout, and Blend If stacks, and asserts
`neutral ⇒ composite_rgba unchanged`.

## compositing/layer-management

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A no-op records no history | Call Merge Down on the bottom layer. | No state is added. | `tst_layer_commands::managementNoOpRecordsNoHistory` |
| One state per command | Run New Group and Merge Down, then undo each. | One state each, and each undo restores. | `tst_layer_commands::managementCommandRecordsOneState` |
| Content-neutral creation records one state without a recomposite | Call `add_layer_in`, check history and `frame_revision`, then undo. | One `New Layer` state, `frame_revision` unchanged, the undo restores. | `tst_edit_refresh::newLayerRecordsOneStateWithoutRecomposite` |

## interop/agentic-control

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Snapshot after an edit | With metrics on, `dispatch("layer_op", add)`, then `dispatch("metrics")`. | An edit record labelled `New Layer`; phases are non-empty. | `tst_control_metrics::snapshotContainsEditRecord` |
| Reset clears the totals | Call `metrics` with `reset: true`, then call `metrics` again. | No phases and no edits. | `tst_control_metrics::resetClearsTotals` |
| Disabled metrics are refused | With metrics off, call `metrics`. | Error code `refused`. | `tst_control_metrics_off::disabledIsRefused` |
| A group is added | `layer_op add_group` at index 0. | One more node, and it is a group. | `tst_control_metrics::addGroupCreatesGroup` |

## verification/tool-profiler

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| New Layer is profiled with a breakdown | Run `scripts/profile-tools.py --scenarios new-layer --metrics` against a release build. | Wall time plus phases, including `layers_panel_refresh`. | manual: drives a built binary over the control socket, and no automated layer launches the profiler |
| Profiling without metrics still works | Run the same scenario without `--metrics`. | Wall time only, exit 0. | manual: same reason |
