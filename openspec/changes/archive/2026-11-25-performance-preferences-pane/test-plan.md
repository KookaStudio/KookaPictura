# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pp_history_default` | Start with no saved value; capture 25 edits | Spin box 20; 20 undo states kept | `tst_performance_preferences::historyStatesDefaultsToTwenty`, `pictura-app::history::tests::depth::capture_keeps_the_state_limit` |
| `pp_history_trim` | 10 edits + a snapshot, set History States to 4 | 4 undo states and the snapshot remain; panel refreshed | `pictura-app::history::tests::depth::lowering_the_limit_drops_the_oldest`, `tst_performance_preferences::loweringHistoryStatesTrimsOpenDocuments` |
| `pp_history_trim_redo` | 10 edits, undo to oldest, limit 3 | Oldest + 3 newer kept, cursor unchanged | `pictura-app::history::tests::depth::at_the_oldest_state_redo_states_go` |
| `pp_history_persist` | Set 50, read preferences back | Saved 50, engine 50 | `tst_performance_preferences::historyStatesPersists` |
| `pp_cache_cap` | Pyramid for 20000² with cap 3 and 8 | 3 levels; full count | `pictura-render::tile_pyramid::tests::cache_levels_cap_the_pyramid` |
| `pp_cache_applies` | 4096² doc, Cache Levels 2 then 8 | 2 display levels, then 5 | `tst_performance_preferences::cacheLevelsRebuildOpenViews` |
| `pp_cache_persist` | Set 6, read back | Saved 6 | `tst_performance_preferences::cacheLevelsPersists` |
| `pp_presets` | Click Tall and Thin, Default, Big and Flat | 4, 8, 8; engine follows | `tst_performance_preferences::presetsSetCacheLevels` |
| `pp_tile_fixed` | Show the page | Tile menu disabled, 256 px | `tst_performance_preferences::cacheTileSizeIsFixed` |
| `pp_scratch_list` | Add two dirs, move second up, remove first | Store and saved list follow | `tst_performance_preferences::scratchVolumesAddReorderRemove` |
| `pp_scratch_limit` | List four volumes | Add disabled | `tst_performance_preferences::scratchVolumesStopAtFour` |
| `pp_scratch_default` | No volume configured | One disabled default row, Remove disabled | `tst_performance_preferences::emptyScratchListShowsTheDefault` |
| `pp_gpu_adapter` | Set an adapter name, then empty | Named, then "No graphics processor found" | `tst_performance_preferences::gpuLineNamesTheAdapter` |
| `sse_scratch_format` | Format 327 680 000 / 2 190 433 320 and unknown | `312.5M/2.04G`, `312.5M/—` | `tst_performance_preferences::scratchSizesFormatLikeCs6` |
| `sse_scratch_menu` | Choose Scratch Sizes | Readout shows used/available | `tst_performance_preferences::scratchSizesMenuShowsAPair` |
| `sse_efficiency` | Readings (0,0)→(900,100)→same→(1000,200) | 90 %, 90 %, 50 % | `tst_performance_preferences::efficiencyCoversTheLatestInterval` |
| `sse_efficiency_menu` | Choose Efficiency, nothing measured | `100%` | `tst_performance_preferences::efficiencyMenuStartsAtHundred` |
| `mg_preference_default` | Start without saved value | 60 | `tst_memory_governor::memoryUsageDefaultsToSixty` |
| `mg_preference_change` | Set 45, read back | 45 | `tst_memory_governor::memoryUsageChangeAppliesAndPersists` |
| `mg_preference_slider` | Move slider to 30 | Spin box and governor 30; ceiling line in MB | `tst_performance_preferences::memorySliderAndCeilingFollow` |
| Pane layout | Open Edit > Preferences > Performance at 100 % and 200 % UI scale | Groups legible, nothing clipped, reads like CS6's pane | manual: visual judgement of layout |
| Zoomed-out redraw with a low Cache Levels | Open a 20000² image, set Tall and Thin, zoom to fit | Image draws correctly (slower), no blank tiles | manual: needs a large image and a person's eye on redraw |
