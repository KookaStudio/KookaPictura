# Test Plan

Covers the `ui/panel-column` delta. The scenarios are widget-state checks, so
they are automated with Qt Test against the real `PicturaMainWindow`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cfd_group_tearoff` | Put the primary column in rail mode, open the Layers flyout, begin a whole-group drag from the flyout, drag above the column and drop | The flyout is no longer visible, one float holds the group's panels, and the source column no longer owns the group | `tst_panels_parity::compactFlyoutClosesWhenGroupTearsOff` |
| `cfd_tab_tearoff` | Open the Layers flyout (Layers, Channels, Paths visible), begin a Channels tab drag from the flyout, drag above the column and drop | The flyout stays visible and still hosts the group with Layers and Paths; one float holds only Channels | `tst_panels_parity::compactFlyoutStaysOpenForTabTearOff` |
| `cfd_empty_tearoff` | Open the Adjustments flyout (only Adjustments visible; Properties is hidden), begin an Adjustments tab drag, drag above the column and drop | The flyout is no longer visible and one float holds only Adjustments | `tst_panels_parity::compactFlyoutClosesWhenTabEmptiesIt` |
