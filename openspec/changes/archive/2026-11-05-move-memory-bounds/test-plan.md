# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mmb_bounded_read` | Read a 90×300 layer with a 41×80 mark whole and over the mark, over a box below it, and over the mark after forking band 1 from a history copy | Box read is the crop; outside reads nothing; no plane mirrored | `pictura-app::a_bounded_buffer_is_the_crop_of_the_whole`, `pictura-app::reading_a_diverged_banded_layer_builds_no_mirror` |
| `mmb_mark_move` | 800×600: fill a 40×40 mark at (40, 40) on a new layer, press, commit (30, 20), press again | Image 40×40 at (40, 40); damage ≤ 80×80; black at (100, 80), white at (45, 45); cache hit at (70, 60) | `tst_damage_bounds::paintedMoveIsBoundedToTheMark` |
| `mmb_damage` | Translate the marked layer by (10, 30), and a solid 8×8 layer by (2, 0) | Damage (20, 70)–(71, 180); solid 0–10 | `pictura-app::a_move_damages_the_mark_before_and_after` |
| `mmb_gate` | With 64 bytes available: press; press with memory, then commit with 64; flip in Free Transform and commit with 64; restore and commit | Each refused with the reason; history and pixels unchanged; no transform session; the restored move commits | `tst_damage_bounds::moveAndTransformRefuseWhatMemoryCannotHold` |
| `mmb_transform_estimate` | Estimate a 10×10 → 20×20 quad and a fractional one | 4·400 + 12·400; 4·110 + 12·110 | `pictura-app::a_transform_commit_charges_its_box_and_the_composite_around_it` |
| `mmb_history_cap` | Rewrite a 64×256 composite 8 times with a 2.5-composite cap; 5 times with no binding cap | Within the cap or 2 states, fewer than 9 states, undo restores 7; 6 states kept | `pictura-app::the_oldest_states_go_when_history_outgrows_its_cap`, `pictura-app::a_roomy_cap_keeps_every_state` |
| `mmb_total_memory` | Parse meminfo text; query the host | `MemTotal` in bytes; available ≤ total | `pictura-codec::meminfo_available_is_parsed_in_bytes`, `pictura-codec::the_host_reports_available_memory` |
| `mmb_large_manual` | On a ~7 GB host, 17000×17000: paint a box on a new layer, then Move, resize and Free Transform it repeatedly | No OOM kill; an edit that cannot fit shows the memory message | manual: a 17000×17000 document does not fit the test runners' memory, and the OOM killer is a property of the real host; `move_mem.py`-style RSS replays at 4250² measure the stages |
