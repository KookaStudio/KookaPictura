# Test Plan

This plan covers `document/history-tile-sharing` and the modified
`document/history-retention` for `tiled-history` (#1215).

**Fixtures.** The history tests use a 512² document with one patterned layer,
so each plane holds four dense 256² tiles. The identity test is driven by a
fixed-seed generator. Every history test also checks the incremental tile census
against a full recount after each capture and restore (`check_census`), so any
drift in its bookkeeping fails whichever test caused it. Checks that assert "no
materialization" are exact under nextest, which runs one process per test.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `hts_reference` | Swap a held tile into a plane and back; capture an edit inside one tile of a multi-tile plane | The plane gets back the old allocation and its generation, with no sample copied; the delta holds only the touched tile | `pictura-core::a_swapped_tile_moves_its_allocation_and_generation`, `pictura-app::a_capture_scans_only_changed_tiles` |
| `hts_identity` | Capture 16 seeded paint, add-layer and visibility edits; undo to the first state, then redo to the last | At every step the live document equals the captured document byte for byte, and its stamps agree with history's | `pictura-app::undo_redo_identity_seeded` |
| `hts_snapshot` | Take a snapshot, edit, then restore the snapshot by capturing its document | Held bytes are unchanged by the snapshot; the restore moves the edited tile into a delta; no plane is materialized | `pictura-app::snapshot_costs_no_pixels` |
| `hts_snapshot_gui` | Fill a marquee on a 600² document, take a snapshot, fill across tile edges, restore it, then undo and redo | The canvas shows the snapshot's pixels, one state named "Snap" is added, undo shows the edit, redo shows the snapshot again | `tst_history_snapshots::snapshotEditRestoreRoundTrip` |
| `hts_snapshot_large` | Run M8's snapshot, restore and record on a 17000² document (and at 4250² in CI), timing them and measuring the heap they add without the display rebuild | At most 16 MiB added and at most 100 ms; no plane materialized (display refresh excluded, #1216) | `pictura-app::snapshot_restore_edit_shares_tiles_full`, `pictura-app::snapshot_restore_edit_shares_tiles`, `pictura-app::snapshot_restore_heap_peak` |
| `hts_counted_once` | A snapshot and a later delta hold the same tile; undo and redo | Held bytes count the tile once and match the recount | `pictura-app::shared_tiles_counted_once`, `pictura-core::a_census_counts_each_kept_allocation_once`, `pictura-core::a_census_does_not_count_what_the_live_document_holds` |
| `hts_budget` | Pin a host ceiling and read the history budget; capture whole-plane edits under a cgroup limit | The budget is half the ceiling; history stays under it | `pictura-core::history_may_hold_half_the_ceiling`, `pictura-app::the_history_cap_is_half_the_ceiling` |
| `hts_drop_order` | Capture eight whole-plane edits under a cap of two and a half planes; then a roomy cap | Oldest states are dropped, one undo remains and restores the previous bytes; a roomy cap keeps every state | `pictura-app::the_oldest_states_go_when_history_outgrows_its_cap`, `pictura-app::a_roomy_cap_keeps_every_state` |
| `hts_memory_max` | Run twenty M6 strokes at 17000² with `python3 scripts/profile-tools.py --scenarios M6x20 --peak --size 12000x12000 --memory-max 4G` | The run is not OOM-killed and history keeps to its budget | manual: needs a systemd user scope with a 4 GiB limit; CI runners cannot host it. At 17000² the New Document gate refuses the setup under 4G (display memory, #1216), so run it at 12000² |
| `hts_purge` | Hold tiles in states and a snapshot; Purge Undo, Purge Histories, then build again and Purge All; also dispatch the Edit › Purge commands | Purge Undo leaves only the snapshot's tile; Purge Histories and Purge All leave none; no undo after a purge | `pictura-app::purge_releases_history_tiles`, `tst_edit_clipboard::purgeCommands` |
| `hr_bounded_retention` | Capture ten one-pixel edits on a 1024² document | One materialized state; retained delta bytes are at most ten 256² tiles and far below one full state | `pictura-app::region_deltas_retain_only_changed_tiles` |
| `hr_geometry_anchor` | Capture a resize, crop, flatten, or layer/mask structure change | The state is a full snapshot and a new anchor | `pictura-app::geometry_change_uses_a_full_anchor_and_still_undoes` |
