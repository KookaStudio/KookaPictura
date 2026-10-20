# Test Plan

One row per spec scenario in this change. New automated checks live in
`crates/pictura-app/cpp/tests/tst_floating_workspaces.cpp`; unchanged scenarios
keep their existing coverage and are marked manual here rather than claiming a
test this change does not touch. Coverage cells contain either test references
or a single `manual: <reason>` clause.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `Session state is restored on launch` | Save a session with floats, quit, relaunch | Layout, brightness, and panel-column state return, including floats | `tst_floating_workspaces::restartRestoresFloats` |
| `Missing session state` | Launch with no state file | Default layout and brightness | manual: existing session-store load path |
| `Corrupt session state` | Launch with an unparseable state file | Defaults, no crash | manual: existing session-store corrupt-load path |
| `Atomic write` | Save the session | Temp file renamed over the previous store | manual: existing session-store save path |
| `m41_session` | Load a schema-4 store | Panel-column defaults, existing keys preserved | `tst_floating_workspaces::schema4StoreKeepsDefaultGroups` |
| `m43_session` | Save and reload a multi-column layout with floats | Sides, order, widths, rail modes, floating state, geometry, and group state restored | `tst_floating_workspaces::restartRestoresFloats` |
| `m43_session` | Load a schema-5 store with no per-column layout | One right-hand column from the legacy per-group state | manual: existing session-store migration path |
| `lpr_v7` | Load a schema-6 store without widths | Default width; legacy `railWidth` seeds the primary | manual: existing UI-persistence width seeding |
| `lpr_v8_railmode` | Load a schema-7 store without rail modes | Legacy `panelRailMode` seeds every column | manual: existing UI-persistence rail-mode seeding |
| `lpr_column_mode` | Change one column's rail mode, restart | Each column keeps its own mode | manual: existing UI-persistence rail-mode check |
| `spr_stale_refused` | Second launch writes; first launch then saves | First save refused; newer store unchanged | `tst_session_store::anotherInstancesNewerWriteIsRefused` |
| `spr_stale_adopted` | Store predates this launch | Save succeeds and stamps this launch's token | `tst_session_store::aStoreFromBeforeTheLaunchIsAdopted` |
| `spr_legacy_adopted` | Store has no writer token | Save succeeds and stamps a token | manual: existing session-store legacy-token path |
| `spr_floats_group` | Float a group, quit, relaunch | Group floats again at its stored rect | `tst_floating_workspaces::restartRestoresFloats` |
| `spr_floats_column` | Float a widget column, quit, relaunch | Column floats again at its stored rect | `tst_floating_workspaces::restartRestoresFloats`, `tst_floating_workspaces::workspaceRoundTripsEmptyFloatingColumn` |
| `spr_floats_tools` | Float the Tools column, quit, relaunch | Tools floats again, content-sized, at its stored position | `tst_floating_workspaces::toolsFloatRestores` |
| `spr_floats_legacy` | Load a store without float geometry | Every panel and column opens docked | `tst_floating_workspaces::legacyStoreDocksEverything` |
| `spr_floats_clamped` | Restore oversized/undersized overlay rects | Position and size clamped to bounds and minimum | `tst_floating_workspaces::restoredFloatIsClamped` |
| `spr_floats_startup_position` | Store a float at the right of a frame with non-default window geometry, restart | Overlay is placed against the restored frame and keeps its right-hand position | `tst_floating_workspaces_restart::restartRestoresFloatAtRightOfRestoredFrame` |
| `m41_iconic` | Enter iconic mode | Narrow strip with dividers, no full content | manual: unchanged scenario; current shell self-test coverage retained |
| `m42_iconic` | Build the iconic strip | Icons exceed the M41 sizes | manual: unchanged scenario; current shell self-test coverage retained |
| `m45_popup_group` | Click a panel icon in the strip | Flyout hosts the whole group, clicked panel current, active icon, restores on close | `tst_floating_workspaces::flyoutHostsWholeGroup` |
| `m42_flyout` | Open a flyout on a right column, then a left column | Opens to the inner side in both cases | manual: unchanged scenario; current shell self-test coverage retained |
| `m43_icon` | Build the iconic strip | Icons exceed the M42 sizes | manual: unchanged scenario; current shell self-test coverage retained |
| `m43_flyout` | Place a flyout from an icon's actual rect | Inner edge meets the button; never crosses to the outer side | `tst_floating_workspaces::flyoutCloseButtonPlacement` |
| `m42_iconic` | Open a panel icon's flyout | That icon renders active/pressed | `tst_floating_workspaces::flyoutHostsWholeGroup` |
| `m44_popupstyle` | Compare a panel docked and in the flyout | Same tab bar, background, and border styling | manual: unchanged scenario; current shell self-test coverage retained |
| `m44_compactdivider` | Build a strip with more than one group | Divider is dark grey, not white | manual: unchanged scenario; current shell self-test coverage retained |
| `m44_elide` | Widen the strip slightly past the icon width | Labels appear and elide | manual: unchanged scenario; current shell self-test coverage retained |
| `m44_compactdrop` | Drop an icon onto/above/below a compact group | Insert into that group | manual: unchanged scenario; current shell self-test coverage retained |
| `m44_compactdrop` | Drop between/above/below groups | New boundary group | manual: unchanged scenario; current shell self-test coverage retained |
| `m44_draghandle` | Drag the compact drag-handle | Whole group drags | manual: unchanged scenario; current shell self-test coverage retained |
| `m47_compact_icon_float` | Drag a compact icon away | Floating overlay follows and commits | manual: unchanged scenario; current shell self-test coverage retained |
| `m47_compact_grip_group` | Drop a compact panel on a grip | New one-panel group above | manual: unchanged scenario; current shell self-test coverage retained |
| `m47_iconic_fixed_width` | Drag the splitter between normal and iconic columns | Iconic width fixed; normal column resizes | manual: unchanged scenario; current shell self-test coverage retained |
| `pc_flyout_persists` | Open a flyout, click elsewhere | Flyout and group remain visible | `tst_floating_workspaces::flyoutSurvivesOutsideClick` |
| `pc_flyout_toggle` | Re-click the icon; click another icon | First closes; second replaces; never two per column | `tst_floating_workspaces::flyoutTogglesAndSwitches` |
| `pc_flyout_escape` | Press Escape while open | Flyout remains visible | `tst_floating_workspaces::escapeDoesNotCloseFlyout` |
| `pc_flyout_group_leaves` | Drag the group out; close its last panel; remove its column | Flyout hidden; no empty overlay | `tst_floating_workspaces::flyoutGroupLeavesNoStaleOverlay`, `tst_floating_workspaces::flyoutFloatingRowDragLeavesNoStale`, `tst_floating_workspaces::floatingRowFlyoutCloseGroupNoStale` |
| `pc_flyout_hide_all` | Press `Tab` with the flyout open | Flyout hidden with all panels | `tst_floating_workspaces::flyoutClosesOnHideAll` |
| `pc_col_float` | Drag a widget column header out | Column floats and follows the cursor; release commits | `tst_workspaces::wholeColumnFloatDoesNotPersistPhantom`, `tst_floating_workspaces::normalColumnFloatKeepsTwoThirdsHeight` |
| `pc_col_float_grip` | Inspect/drag a floating column grip | Grip present; width floored | manual: unchanged scenario; current shell self-test coverage retained |
| `pc_col_float_height` | Tear off a normal docked column | Initial height is less than docked and about two thirds | `tst_floating_workspaces::normalColumnFloatKeepsTwoThirdsHeight` |
| `pc_col_float_iconic_initial` | Tear off an already-iconic column | Initial height is about two thirds, not the icon minimum | `tst_floating_workspaces::iconicColumnFloatKeepsTwoThirdsHeight` |
| `pc_col_float_iconic_snap` | Toggle rail mode to iconic while floating | Overlay snaps to icon-row height and narrow width | `tst_floating_workspaces::iconicColumnSnapsWhenRailToggled` |
| `pc_col_redock` | Drop a floating column on an edge | Column re-places; overlay removed | manual: unchanged scenario; current shell self-test coverage retained |
| `pc_col_float_child` | Drag an in-window column overlay header | Moves parent-relative and stays clamped | manual: unchanged scenario; current shell self-test coverage retained |
| `pc_float_icon_click` | Click a floating row icon | Persistent flyout opens; outside clicks do not close it | `tst_floating_workspaces::floatingIconRowOpensFlyout` |
| `pc_float_icon_grip` | Drag the floating row's grip | Whole group drags through the docked grammar | manual: unchanged scenario; current shell self-test coverage retained |
| `pc_float_icon_single_header` | Open a floating row's group in the flyout | Float top bar and close control hidden; one header | `tst_floating_workspaces::floatingIconRowShowsOneHeader` |
| `m41_session` | Load a schema-4 store | Panel-column defaults, existing keys preserved | `tst_floating_workspaces::schema4StoreKeepsDefaultGroups` |
| `m43_session` | Save and reload a multi-column layout with floats | Sides, order, widths, rail modes, floating state, geometry, and group state restored | `tst_floating_workspaces::restartRestoresFloats` |
| `m43_session` | Load a schema-5 store with no per-column layout | One right-hand column from the legacy per-group state | manual: existing session-store migration path |
| `lpr_v7_width` | Load a schema-6 store without widths | Default width; legacy `railWidth` seeds the primary | manual: existing UI-persistence width seeding |
| `lpr_v7_railmode` | Load a schema-7 store without rail modes | Legacy `panelRailMode` seeds every column | manual: existing UI-persistence rail-mode seeding |
| `lpr_column_railmode` | Change one column's mode, restart | Each column keeps its own mode | manual: existing UI-persistence rail-mode check |
| `pc_flyout_close_placement` | Open a flyout and inspect the corner | Chevrons-right close control left of the widget menu; actual-width placement | `tst_floating_workspaces::flyoutCloseButtonPlacement` |
| `pc_flyout_close_action` | Activate the close control | Flyout hides; group returns to its owner slot | `tst_floating_workspaces::flyoutCloseButtonHidesFlyout` |
| `pc_flyout_close_visibility` | Inspect the group docked, floating, and in the flyout | Close control only visible in the flyout | `tst_floating_workspaces::flyoutCloseButtonOnlyInFlyout` |
| `pc_flyout_resize_left` | Drag a right-hand flyout's left edge | Width changes; right edge fixed | `tst_floating_workspaces::flyoutResizesFromLeftEdge` |
| `pc_flyout_resize_right` | Drag a left-hand flyout's right edge | Width changes; left edge fixed | `tst_floating_workspaces::flyoutResizesFromRightEdge` |
| `pc_flyout_resize_min` | Drag below the shared minimum | Width stops at the minimum | `tst_floating_workspaces::flyoutResizeKeepsMinimum` |
| `pc_flyout_resize_floating_row` | Drag a floating-row flyout's canvas edge | Width changes; opposite edge fixed | `tst_floating_workspaces::flyoutResizesBesideFloatingRow` |
| `pc_flyout_resize_open_width` | Open a flyout | Initial width is at least the shared minimum | `tst_floating_workspaces::flyoutOpensAtMinimumWidth` |
| `pc_flyout_child_bounds` | Open/resize a forced-child flyout | Rect stays inside the frame | `tst_floating_workspaces::flyoutChildModeStaysInFrame` |
| `pc_float_header_surface` | Compare a floating top bar and docked header | Same panel-header background | `tst_floating_workspaces::floatHeaderMatchesDockedHeader` |
| `pc_float_corner_menu_painted` | Grab a floating group and sample the corner menu | Painted and clickable | `tst_floating_workspaces::floatCornerMenuPainted` |
| `wsp_floats_remembered` | Modify a workspace with floats, switch away and back | Floats return at their remembered geometry | `tst_floating_workspaces::workspaceSwitchRemembersFloats` |
| `wsp_floats_roundtrip` | Reload a user workspace created with floats | Floating groups and columns return with geometry | `tst_floating_workspaces::workspaceRoundTripsGroupFloat`, `tst_floating_workspaces::workspaceRoundTripsColumnFloat` |
| `wsp_floats_legacy` | Apply a workspace without float markers | Every widget panel docks | `tst_floating_workspaces::workspaceWithoutFloatsDocksEverything` |
| `wsp_floats_tools` | Apply a workspace while Tools floats | Tools keeps its floating state and position | `tst_floating_workspaces::toolsFloatSurvivesWorkspaceApply` |
