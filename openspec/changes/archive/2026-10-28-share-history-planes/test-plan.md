# Test plan — share-history-planes

One row per scenario in
`specs/document/history-retention/spec.md`. Coverage names the test that proves
the row; every row runs in CI.

Run with:

```bash
cargo nextest run -p pictura_app history
cargo test -p pictura_app large_document_structural_edit_profile -- --ignored --nocapture
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `hr_structural_sharing` | Capture Open on an N-layer document, then insert a New Layer and capture twenty times without changing other pixels; read retained plane bytes | At most the base state plus twenty added-layer plane copies, not twenty documents | `pictura-app::structural_captures_retain_only_new_planes` |
| `hr_structural_exact` | Interleave layer add, delete, reorder, and paint captures, then undo, redo, and jump across segment anchors against a full-snapshot reference | Every restored document and selection equals its reference byte for byte | `pictura-app::live_restores_match_a_full_snapshot_history`, `pictura-app::structural_capture_with_a_duplicated_layer_round_trips` |
| `hr_live_private` | After every structural capture and restore, compare each live tracked plane's allocation with every retained history state | No tracked plane of the live document shares an allocation with a tracked plane of any history state | `pictura-app::structural_captures_retain_only_new_planes`, `pictura-app::undoing_a_structural_edit_keeps_the_live_planes` |
| `hr_restore_reuses_live` | Add a layer, capture, hold the live document, undo | Surviving layer planes and the composite share the pre-undo live allocations; only target-changed planes are copied | `pictura-app::undoing_a_structural_edit_keeps_the_live_planes` |

The ignored `large_document_structural_edit_profile` is the performance
evidence for the issue (add+capture, undo, retained bytes at 6000×4000 starting
with twenty full-canvas layers and measuring twenty New Layer captures); it
prints the issue's targets (add+capture <20 ms, undo <30 ms, retained <5 GB)
beside the measurements rather than asserting them, because CI hardware varies.
The deterministic byte bound is asserted by the unit test above.
