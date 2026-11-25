# Test plan: move-memory-recovery

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cr_recovery_cow_reserve` | Queue a recovery snapshot of a 196 MiB flat document in a room below the document's plane bytes but above its tables. | The snapshot is queued, not refused. | `pictura-app::a_cow_snapshot_reserves_only_its_tables` |
| `hts_flat_plane_gate` | Tile a flat 16 MiB plane in a room that cannot hold the tiled copy. | The plane stays flat. | `pictura-app::a_flat_plane_stays_flat_when_the_room_is_small` |
| `hts_tiled_when_room` | Tile a flat plane with room for the tiled copy. | The plane becomes tiled. | `pictura-app::opening_a_document_shares_tiles_with_history` |
