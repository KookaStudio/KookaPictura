# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_menu_entry` | Launch with no document; open Help ▸ Developer | `Debug Toolset` is enabled, unchecked, no panel | `tst_debug_toolset::menuEntryIsCheckableAndOffAtStartup` |
| `dbg_not_persisted` | Enable, drag the panel, restart | Toolset off, panel at default place, no setting written | `tst_debug_toolset::stateIsNotPersisted` |
| `dbg_off_no_collection` | Toggle on then off | Timer inactive, no stored sample | `tst_debug_toolset::disabledRunsNothing` |
| `dbg_toggle_panel` | Choose Debug Toolset twice | Panel shows then hides; check mark follows | `tst_debug_toolset::toggleShowsAndHidesPanel` |
| `dbg_drag` | Drag the header past the window edge | Panel follows the pointer, stays inside the window | `tst_debug_toolset::headerDragStaysInsideWindow` |
| `dbg_interval` | Enable, watch values | Refresh every 2 s | `tst_debug_toolset::refreshIntervalIsTwoSeconds` |
| `dbg_first_sample` | Enable, read rows before and after the second refresh | `…` on rate rows, then numbers | `tst_debug_toolset::rateRowsWaitForSecondSample` |
| `dbg_process_parse` | Feed status/stat fixtures | RAM, swap, threads, CPU % match | `tst_debug_toolset::parsesProcessRows` |
| `dbg_gpu` | Format a snapshot with and without a GPU frame time | ms value, or `not measured` | `tst_debug_toolset::gpuRowReportsMeasuredOrNot` |
| `dbg_thread_groups` | Group fixture tasks | Right groups, CPU order | `tst_debug_toolset::threadsGroupByNameAndRank` |
| `dbg_thread_names` | Run a rayon task and an open job, read thread names | `pictura-rayon-` and `pictura-open` prefixes | `pictura-app::worker_threads_carry_group_names` |
| `dbg_memory_swap_parse` | Feed meminfo/swaps/vmstat fixtures | Totals, used, cached, swap path and I/O match | `tst_debug_toolset::parsesMemoryAndSwap` |
| `dbg_cpu_load_parse` | Feed stat and loadavg fixtures | Per-CPU % and load match | `tst_debug_toolset::parsesCpuAndLoad` |
| `dbg_platform_rows` | Enable the panel | OS, arch, desktop, session rows non-empty | `tst_debug_toolset::platformRowsArePresent` |
| `dbg_build_placeholder` | Enable the panel | Version row reads `n/a (placeholder)` | `tst_debug_toolset::versionRowIsPlaceholder` |
| Visual layout | Enable on a real X11 and Wayland desktop | Panel placement and legibility match the issue's mock-up | manual: offscreen Qt cannot judge appearance |
| Real GPU row | Launch with `PICTURA_METRICS=1`, paint on a Vulkan device | GPU row shows a changing frame time | manual: needs a real GPU |
