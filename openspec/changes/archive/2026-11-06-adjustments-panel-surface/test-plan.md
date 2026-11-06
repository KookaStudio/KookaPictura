# Test Plan

Covers every scenario in the `ui/adjustments-panel` delta for
`adjustments-panel-surface`. Both scenarios are automated in
`tst_adjustments_panel`; no manual rows are expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `adj_panel_surface` | Show a `PicturaMainWindow`, activate the Adjustments tab, sample a background pixel inside the panel; activate the Styles tab in the same group, sample the same relative point | Both samples equal each other and the theme's `surface` token; the Adjustments sample is not `surface-field` | `tst_adjustments_panel::panelSurfaceMatchesStylesPanel` |
| `adj_panel_surface_theme` | With the Adjustments tab active, `Theme::apply()` a different brightness level, process events, and sample the panel background; restore the default theme | The sample equals the new `surface` token | `tst_adjustments_panel::surfaceFollowsTheme` |
