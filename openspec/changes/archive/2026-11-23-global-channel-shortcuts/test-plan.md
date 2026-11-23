# Test Plan

One row per scenario in the delta spec. Every row is automated.

## ui/channels-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| cp_shortcut_individual | Open an RGB document, focus the canvas, and press `Ctrl+3`, `Ctrl+3`, `Ctrl+4`, `Ctrl+4`, `Ctrl+5`, `Ctrl+5`. | Each press flips only that colour row's eye; the canvas channel mask follows (`0x6`/`0x7`, `0x5`/`0x7`, `0x3`/`0x7`). | `tst_channels_panel::channelShortcutsToggleFromTheCanvas` |
| cp_shortcut_composite | Hide the Red channel with `Ctrl+3`, then press `Ctrl+2`. | All four eyes are visible again and the canvas mask is `0x7`. | `tst_channels_panel::channelShortcutsToggleFromTheCanvas` |
