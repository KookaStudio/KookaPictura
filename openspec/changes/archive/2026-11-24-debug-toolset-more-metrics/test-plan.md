# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_activity_parse` | Sample status/stat/io/task fixtures twice | Anon/file RAM, files, rates match | `tst_debug_toolset::parsesActivityRows` |
| `dbg_activity_rows` | Refresh twice | Files, disk, faults, ctx rows filled | `tst_debug_toolset::panelShowsActivityRows` |
| `dbg_jobs` | Sample fixture tasks | Open 12 s, Save 3 s, else `none` | `tst_debug_toolset::jobsComeFromNamedThreads` |
| `dbg_budget_parse` | Parse a status report and an empty one | All values, then none | `tst_debug_toolset::parsesMemoryBudgetReport` |
| `dbg_budget_panel` | Sample with a fixture report | Governor row shows used, ceiling, pressure; `n/a` without | `tst_debug_toolset::budgetRowsShowTheReport` |
| `dbg_gpu_fdinfo_parse` | Sample DRM fdinfo fixtures twice | Clients counted once, 50 % busy | `tst_debug_toolset::parsesDrmClientInformation` |
| `dbg_gpu_system_parse` | Sample sysfs GPU fixtures | Busy and VRAM match; none when empty | `tst_debug_toolset::parsesSystemGpuSysfs` |
| `dbg_timing` | Format snapshots with and without edits | p50, p95, last; `not measured` | `tst_debug_toolset::timingTextHasPercentiles` |
| `dbg_group_chart` | Refresh three times, then close | GUI line 2 points, then empty | `tst_debug_toolset::groupChartGrowsAndClears` |
| Real GPU rows | Run on an AMD or Intel GPU and render | Process busy and memory move with load | manual: needs a real GPU |
| Jobs on screen | Start a save and watch Activity | Save listed with a rising time | manual: needs a live job |
