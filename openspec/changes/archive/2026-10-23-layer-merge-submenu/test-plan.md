# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| All top-level menus are present | Build a frame and read the menu-bar titles. | File, Edit, Image, Layer, Type, Select, Filter, View, Window, Help in that order. | `tst_command_tree::menus` |
| Unimplemented leaf is disabled | Inspect a filter leaf the table keeps without a handler. | The leaf is described as not implemented and not enabled. | `tst_filter_menu::stubEntriesStayDisabled` |
| cr_layer_merge_submenu | Open the Layer menu and inspect its direct actions and the `Merge` submenu. | The Layer menu has one `Merge` submenu; it holds Merge Layers, Merge Visible, Stamp Visible, Stamp Selected, Flatten Image, Merge Clipping Mask in that order, and none of the six is a direct Layer action. | `tst_merge_commands::layerMergeSubmenu` |
