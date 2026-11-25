# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_title` | Enable the toolset | Header reads `Pictura Metrics` | `tst_debug_toolset::headerReadsPicturaMetrics` |
| `dbg_two_columns` | Enable the toolset | Budget, Activity, Platform right of Usage; Version left; panel shorter than stacked | `tst_debug_toolset::tablesSitInTwoColumns` |
| Fits a laptop screen | Enable on a 1366x768 and a 1920x1080 display | Whole panel visible without scrolling | manual: needs real screen sizes |
