# Test Plan

Covers every scenario in the `ui/application-shell` delta for
`footer-message-replaces-readouts`. Both scenarios are automated in
`tst_notification_status_bar`; no manual rows are expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `las_status_message_replaces_readouts` | Open a document in a `PicturaMainWindow`, confirm the zoom, size, backend, view-options, and tool-hint readouts are visible, then `showMessage()` and process events; finally `clearMessage()` | With the message shown every readout is hidden and the banner rect intersects no visible direct child of the bar (the permanent Search hint included); after `clearMessage()` every readout is visible again | `tst_notification_status_bar::messageReplacesReadouts` |
| `las_status_message_survives_refresh` | With a document open, `showMessage()`, then change the canvas zoom (which calls `updateStatus()`) and process events | The message is unchanged, every readout is still hidden, and the banner rect still intersects no visible footer child | `tst_notification_status_bar::messageSurvivesAStatusRefresh` |
