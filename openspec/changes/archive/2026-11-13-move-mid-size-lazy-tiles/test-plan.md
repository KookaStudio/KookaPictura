# Test plan

This plan covers `tools/move-commit-responsiveness` for
`move-mid-size-lazy-tiles`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcr_deferred` | Commit a canvas-sized layer on an authoritative document at or above 16 MP with a dirty union at or above 16 MP | The commit returns with the composite pending; the landed composite equals a full composite and history gained one Move state | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_deferred_pyramid` | Land a whole-document move composite on an authoritative document over 16 MP | The canvas presents a coarse level, then the exact level after `pyramidSettled` | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_small_sync` | Commit a Move on a document below 16 MP | The composite is current when the commit returns and nothing is pending | `tst_large_document::aSmallMoveCommitIsSynchronous` |
| `mcr_busy_sync` | Run a large Move commit while a live session, a pending preview or a soloed mask owns display state | The commit composites synchronously | `pictura-app::a_busy_display_does_not_defer` |
| `mcr_layered_lazy` | Release a large Move on a layered RGB 8-bit document with a presented view | The preview stays pending and `moveCommitted` arrives only after the presented tiles are current | `tst_large_document::aLargeMoveCommitLeavesTheTilesStale`, `pictura-app::a_layered_large_move_marks_tiles_instead_of_deferring` |
| `mcr_layered_preview` | Release the same Move and inspect the canvas before the tiles land | The preview is still presented and drops on `moveCommitted` | `tst_large_document::aLargeMoveCommitLeavesTheTilesStale` |
| `mcr_layered_mid` | Release a Move with a dirty union of about 5.8 MP on a 4200 px layered RGB document | The commit is pending with the preview presented, and `moveCommitted` arrives once, after the tiles land | `tst_large_document::aMidSizedMoveCommitWaitsForTheTiles`, `pictura-app::a_mid_sized_layered_move_still_marks_tiles` |
| `mcr_lazy_chain` | Commit two Moves on a 4200 px layered RGB document with no event loop between them | No `moveCommitted` after the second commit; the preview stays; one `moveCommitted` arrives when the tiles land; the stale presented tiles show the first Move's result | `tst_large_document::aSecondMoveInheritsThePendingWait`, `pictura-app::a_lazy_move_paints_its_preview_into_the_stale_tiles`, `pictura-render::a_stand_in_paints_only_stale_tiles` |
| `mcr_transform_lazy` | Scale a duplicated layer down with Free Transform on a 4200 px layered RGB document at 1/8 zoom and commit | `moveCommitted` arrives once at the commit, nothing is pending, no preview stays; the stale tiles show the reduced result | `tst_large_document::aLayeredTransformShowsItsResultAtOnce`, `pictura-app::a_reduced_stand_in_shows_the_document_in_the_stale_tiles` |
