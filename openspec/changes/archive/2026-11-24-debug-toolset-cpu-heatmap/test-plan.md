# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_cpu_heat_colours` | Give 0, 50 and 100 % loads, grab the widget | Blue, in-between, red squares | `tst_debug_toolset::heatmapColoursRunColdToHot` |
| `dbg_cpu_heat_rows` | Give 4, 8, 9, 16 loads | 1, 1, 2, 2 rows, every square inside the widget | `tst_debug_toolset::heatmapUsesOneOrTwoRows` |
| `dbg_cpu_heat_panel` | Refresh twice, then close the panel | Empty, one load per CPU, empty | `tst_debug_toolset::panelFeedsHeatmapFromSampler` |
| Heat legibility | Load a few cores and watch the matrix | Busy cores read hot at a glance | manual: needs a live load and a person's eye |
