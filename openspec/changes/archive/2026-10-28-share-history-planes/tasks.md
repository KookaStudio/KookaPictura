# Tasks

## 1. Tests

- [x] 1.1 Add the test-only `History::retained_plane_bytes()` that sums one length per distinct `Plane` allocation across states, the current state, the pinned brush source, and named snapshots; write `structural_captures_retain_only_new_planes` (Open + twenty New Layers) asserting the bound and no live plane shared with any full state; expected red until 2.2 lands; verify with `cargo nextest run -p pictura_app structural_captures_retain_only_new_planes`
- [x] 1.2 Write `undoing_a_structural_edit_keeps_the_live_planes`: after an add capture and undo, the surviving layers and the composite still share the pre-undo live allocations and live shares nothing with history; expected red until 2.3 lands; verify with `cargo nextest run -p pictura_app undoing_a_structural_edit_keeps_the_live_planes`
- [x] 1.3 Write `structural_capture_with_a_duplicated_layer_round_trips`: duplicate a layer (two live planes, one stamp), add a layer, capture, undo, and assert byte-exact restoration and no live/history sharing; verify with `cargo nextest run -p pictura_app structural_capture_with_a_duplicated_layer_round_trips`
- [x] 1.4 Extend `live_restores_match_a_full_snapshot_history` with reorder steps interleaved with add, delete, and paint, and assert no live/history plane sharing after every in-place step; verify with `cargo nextest run -p pictura_app live_restores_match_a_full_snapshot_history`
- [x] 1.5 Write `a_capture_after_a_duplicate_stays_a_delta`: after a duplicate capture, paint layer 2 and the composite, capture, and assert the anchor count is unchanged, the undo is `Restored::Region`, the duplicate layer's byte is unchanged, and live shares no plane with the current; this is regression coverage for the stamp retirement: it fails only if `shared_copy` lets duplicate stamps survive; verify with `cargo nextest run -p pictura_app a_capture_after_a_duplicate_stays_a_delta`

## 2. Plane sharing

- [x] 2.1 Add `planes::shared_copy(source, donor)`: take `donor`'s plane for each equal nonzero stamp that is not the same allocation as the source plane, copy the rest, then stamp the result and `source` equal; verify `cargo nextest run -p pictura_app history`
- [x] 2.2 Use it in the structural capture arm of `capture_live`, building the new current before `fill_anchor` so the delta reversal forks only touched planes; verify tasks 1.1, 1.3, and 1.4 pass
- [x] 2.3 Use it in the cross-anchor install of `restore_live` so the live document keeps its own stamp-equal planes; verify tasks 1.2, 1.3, and 1.4 pass

## 3. Profile and gate

- [x] 3.1 Add ignored `large_document_structural_edit_profile` (6000×4000, twenty full-canvas layers): print add+capture and undo times, retained bytes after twenty New Layers, and the issue's targets (20 ms add+capture, 30 ms undo, 5 GB retained); verify with `cargo test -p pictura_app large_document_structural_edit_profile -- --ignored --nocapture`
- [ ] 3.2 Run `cargo fmt --all` and `bash scripts/verify-fast.sh` (it includes `check-test-plan.py`, `check-openspec-archived.sh`, clippy, the test report, and `openspec validate --all --strict`)
- [x] 3.3 Before the PR, archive the change with `openspec archive share-history-planes` so only `archive/` remains; this syncs the delta into `openspec/specs/document/history-retention/spec.md`. Do not run it during implementation.
