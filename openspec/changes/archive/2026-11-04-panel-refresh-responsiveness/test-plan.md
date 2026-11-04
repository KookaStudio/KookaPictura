# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pr_no_type_no_scan` | Reset the fallback flag; create a document without type; add a layer; wait out the debounce | Fallbacks not registered | `tst_edit_refresh::aRefreshWithoutTypeSkipsTheFontScan` |
| `pr_type_doc_registers` | Commit a type layer; reset the fallback flag; add a layer; wait for the debounce | Fallbacks registered | `tst_edit_refresh::aRefreshOfATypeDocumentRegistersTheFallbacks` |
| `pr_layout_registers` | Reset the fallback flag; ask for a type preview rect in a document without type; commit a type layer, reset again, and rasterize it | Fallbacks registered after each | `tst_edit_refresh::typeLayoutRegistersTheFallbacks` |
| `pr_has_type_depth` | Check a blank document, a top-level type layer, an empty group, and the type layer moved into the group | false, true, false, true | `pictura-render::has_type_layers_finds_type_at_any_depth` |
| `pr_agent_provider` | Refresh the commands of a window with an Agent Control; dispatch Edit > Preferences > Agent Control | Enabled; the Agent Control page opens | `tst_preferences_agent::menuOpensThePage` |
| `pr_large_manual` | `python3 scripts/profile-tools.py --size 16507x16196 --scenarios brush-40,marquee,new-layer,duplicate --timing-log /tmp/t.log`; read `cxx_refreshPanels` lines | Every line ≤ 30 ms, the first (with retarget) included | manual: the full-size document needs more memory than the test runners have, and wall time depends on the machine |
