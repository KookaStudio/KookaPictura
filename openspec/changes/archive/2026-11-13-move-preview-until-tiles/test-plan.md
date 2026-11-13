# Test plan

This plan covers `tools/move-commit-responsiveness` for
`move-preview-until-tiles`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcr_deferred` | Commit a canvas-sized layer on an authoritative document at or above 16 MP with a dirty union at or above 16 MP | The commit returns with the composite pending, and when it lands the composite equals a full composite and history gained one Move state | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_deferred_pyramid` | Land a whole-document move composite on an authoritative document over 16 MP | The canvas presents a coarse level, and after `pyramidSettled` it presents the exact level with bytes equal to a synchronous rebuild | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_small_sync` | Commit a Move on a document or dirty union below 16 MP | The composite is current when the commit returns and nothing is pending | `tst_large_document::aSmallMoveCommitIsSynchronous` |
| `mcr_busy_sync` | Run a large Move commit while a live session, a pending preview or a soloed mask owns display state | The commit composites synchronously | `pictura-app::a_busy_display_does_not_defer` |
| `mcr_preview` | Release a large Move that defers its composite or its presented tiles | The canvas keeps the drag preview and drops it when `moveCommitted` arrives | `tst_large_document::aLargeMoveCommitLandsOffThread`, `tst_large_document::aLargeMoveCommitLeavesTheTilesStale` |
| `mcr_layered_lazy` | Release a large Move on a layered RGB 8-bit document with a presented view | The commit returns with the preview pending, requests the presented tiles, and reports `moveCommitted` only after they are current | `tst_large_document::aLargeMoveCommitLeavesTheTilesStale` |
| `mcr_layered_preview` | Release the same Move and inspect the canvas before the tiles land | The moved layer is already at its new position and the old position shows the backdrop; the preview drops when `moveCommitted` arrives | `tst_large_document::aLargeMoveCommitLeavesTheTilesStale` |
| `mcr_layered_lazy_decision` | Ask whether a large layered RGB Move leaves its tiles lazy | The lazy path is chosen; a test delay still forces the deferred path | `pictura-app::a_layered_large_move_marks_tiles_instead_of_deferring` |
