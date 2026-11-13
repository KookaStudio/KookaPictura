# Test plan

This plan covers `tools/move-commit-responsiveness` for
`move-commit-lazy-tiles`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcr_deferred` | Commit a canvas-sized layer on an authoritative document at or above 16 MP with a dirty union at or above 16 MP | The commit returns with the composite pending, and when it lands the composite equals a full composite and history gained one Move state | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_deferred_pyramid` | Land a whole-document move composite on an authoritative document over 16 MP | The canvas presents a coarse level, and after `pyramidSettled` it presents the exact level with bytes equal to a synchronous rebuild | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_small_sync` | Commit a Move on a document or dirty union below 16 MP | The composite is current when the commit returns and nothing is pending | `tst_large_document::aSmallMoveCommitIsSynchronous` |
| `mcr_busy_sync` | Run a large Move commit while a live session, a pending preview or a soloed mask owns display state | The commit composites synchronously | `pictura-app::a_busy_display_does_not_defer` |
| `mcr_layered_lazy` | Release a large Move on a layered RGB 8-bit document with no test delay | The commit returns without a worker composite, reports `moveCommitted`, records one `Move Layer` state, and marks the dirty display tiles stale | `tst_large_document::aLargeMoveCommitLeavesTheTilesStale`, `pictura-app::a_layered_large_move_marks_tiles_instead_of_deferring` |
