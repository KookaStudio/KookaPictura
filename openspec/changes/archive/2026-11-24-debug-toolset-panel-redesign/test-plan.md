# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_appearance` | Enable the toolset | Light-yellow background, dark text | `tst_debug_toolset::panelIsLightYellowWithDarkText` |
| `dbg_table` | Enable the toolset, read the groups | Titled tables, each metric's label and value in its group | `tst_debug_toolset::rowsSitInTitledGroupTables` |
| `dbg_close` | Click the close button | Hidden, timer stopped, no sample or history, menu unchecked | `tst_debug_toolset::closeButtonStopsCollection` |
| `dbg_drag_icon` | Enable the toolset | A drag icon centred in the header | `tst_debug_toolset::dragIconIsCentredAtTop` |
| `dbg_resize_grip` | Enable, resize larger | Grip in the bottom-right corner, new size taken | `tst_debug_toolset::sizeGripResizesFromBottomRight` |
| `dbg_charts` | Refresh 3 then 70 times | Charts below Busiest, series 3 and 2 points, capped at 60 | `tst_debug_toolset::historyChartsSitUnderBusiest` |
| `dbg_toggle_panel` | Choose Debug Toolset twice | Shows then hides; check follows | `tst_debug_toolset::toggleShowsAndHidesPanel` |
| `dbg_drag` | Drag the header far left of the main window | Panel ends outside the main window | `tst_debug_toolset::headerDragMovesPanelOutsideWindow` |
| `dbg_float_window` | Enable the toolset | Top-level tool window parented to the main window | `tst_debug_toolset::panelIsAFloatingToolWindow` |
| `dbg_process_parse` | Feed status/stat fixtures | RAM, swap, threads, CPU % match | `tst_debug_toolset::parsesProcessRows` |
| `dbg_gpu` | Format a snapshot with and without a GPU time | ms value, or `not measured` | `tst_debug_toolset::gpuRowReportsMeasuredOrNot` |
| `dbg_thread_groups` | Group fixture tasks | Right groups, CPU order | `tst_debug_toolset::threadsGroupByNameAndRank` |
| `dbg_thread_names` | Run a rayon task and an open job | `pictura-rayon-` and `pictura-open` prefixes | `pictura-app::worker_threads_carry_group_names` |
| Real window manager drag | Drag the panel off the app on X11 and Wayland | Moves smoothly, stays above the app, closes with it | manual: needs a real window manager |
| Chart legibility | Run a heavy filter with the panel on | RAM and CPU spikes are visible | manual: needs a live workload and a person's eye |
