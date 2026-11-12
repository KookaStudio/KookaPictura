# Test plan

This plan covers `commit-spill-eviction` (#1290). It modifies
`document/tile-eviction` and `interop/agentic-control`.

**Fixtures.**
- The `pictura-core` tests use the eviction `fixture` (serial, fresh scratch
  volume, eviction off) and `pin_host_at`, a 1 GiB cgroup with a 768 MiB
  ceiling.
- The RAM-only host in `te_urgent_hard` pins `total` with no cgroup.
- The clock tests run single-body passes (`run_pass(1)`) until a condition
  holds. The registry is sharded by thread, so the order bodies are met in is
  not fixed. Cold filler planes keep the hot class out of reach.
- New tiles are written one at a time on the test thread, because the pinned
  host reading belongs to that thread.
- The Qt test holds the commit job with `transform_set_delay`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `te_order` | History-held and live bodies; run a pass needing less than the history bodies hold | Only history-held bodies are packed | `pictura-core::history_tiles_are_evicted_before_live_ones` |
| `te_thresholds` | Pin the host at 99 %, then 101 % of the ceiling; run a step each time | Nothing at 99 %; at 101 %, frees down to ≤ 90 % | `pictura-core::eviction_starts_above_the_ceiling_and_stops_at_ninety` |
| `te_clock_hot` | Read A; read B under a hold; single-body passes until A goes; drop the hold, read the remaining filler under a new hold; passes until B goes | A survives the first pass; B stays while A goes; after the drop B goes before any of the hot filler | `pictura-core::second_chance_and_hot_holds` |
| `te_hot_last` | Read every tile of a plane under a live hold, pin one; run a pass needing every tile | Hot tiles are evicted; the pinned one stays resident | `pictura-core::hot_tiles_are_evicted_last` |
| `te_hold_closed` | Read A under a hold, close it, keep it; read B; single-body passes until B goes | B goes while A stays resident; nothing is stamped once the hold closes | `pictura-core::a_closed_hold_keeps_only_what_it_saw` |
| `te_urgent` | Pin the host at 130 %; allocate a new dense tile | Cold tiles are packed on the allocating thread | `pictura-core::urgent_pressure_evicts_on_the_allocating_thread` |
| `te_urgent_hard` | Cgroup host at 101 %: allocate; RAM-only host at 110 %: allocate | The first evicts cold tiles; the second does not | `pictura-core::a_hard_limit_evicts_at_the_ceiling` |
| `te_alloc_estimate` | Pin 95 % and step; pin 101 % without a step; allocate 48 MiB of dense tiles | The allocating thread evicts cold tiles | `pictura-core::an_allocation_burst_is_caught_between_passes` |
| `te_display_drop` | Pin the host over the ceiling; run a pass | The display-drop epoch advances before any tile is packed | `pictura-core::a_pass_asks_the_display_to_drop_first`, `pictura-app::the_display_cache_drops_on_request` |
| `te_view_hold` | Build the presented view's tiles of a layered document; read a separate plane; pass needing one tile; then change the view and pass again | The separate plane's tile is evicted while the view's layer tiles stay; after the view change those go too | `pictura-app::the_view_holds_only_the_tiles_it_built` |
| `te_cgroup_commit` | 17000² document, full-layer gradient, `MemoryMax=2G`, scratch on disk; Free Transform ×1.5, then ×1.0 + 15° in a fresh run | Each lands one "Free Transform" state; no OOM kill; scratch holds spilled tiles | manual: needs a 289 MP document, a cgroup limit and several GB of scratch, too large and host-dependent for CI |
| `ac_guard_cancels` | Dispatch Image Size | `refused`; dialog gone; size unchanged | `tst_control_modal_guard::imageSizeIsRefused` |
| `ac_guard_every_command` | Dispatch every implemented, enabled command | Every call returns; no dialog left visible | `tst_control_modal_guard::everyCommandAnswers` |
| `ac_guard_outside_request` | Show a dialog with no request running | Dialog stays visible | `tst_control_modal_guard::dialogOutsideARequestStaysOpen` |
| `ac_guard_job_progress` | 4200² document, layer duplicated, job held 800 ms; `layer_op` `free_transform` at 50 % | `ok`; `jobProgress` was seen; history gains one "Free Transform" state | `tst_control_modal_guard::aJobProgressWindowRunsToTheEnd` |
