# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ac_serial_order` | Write `open` then `status` and `list_layers` in one burst over a socket | Replies in id order; `status` sees the opened document | `tst_control_serial::repliesKeepRequestOrder` |
| `ac_serial_queue_full` | Write 70 `status` lines while the first runs | Excess answered `refused`; the rest ok | `tst_control_serial::queueOverItsLimitIsRefused` |
| `ac_serial_connections` | Connect five sockets | Fifth closed; the first four answer `status` | `tst_control_serial::fifthConnectionIsClosed` |
| `ac_serial_disconnect` | Send `open`, disconnect at once, then connect and send `status` | No crash; new client served | `tst_control_serial::disconnectDuringARequestIsSafe` |
| `ac_guard_cancels` | Dispatch `image.image.size` with the denylist not covering it | `refused`; dialog gone; size unchanged | `tst_control_modal_guard::imageSizeIsRefused` |
| `ac_guard_every_command` | Loop over `describe()`, dispatching each implemented, enabled command on a fresh document | Every call returns; no top-level dialog left visible | `tst_control_modal_guard::everyCommandAnswers` |
| `ac_guard_outside_request` | Show a dialog with no request running, then pump events | Dialog still visible | `tst_control_modal_guard::dialogOutsideARequestStaysOpen` |
| `ac_save_no_dialog` | Save a document to `.png`, add a layer, `save` | `refused`; file bytes unchanged | `tst_control_limits::saveNeedingAFormatDialogIsRefused` |
| `ac_endpoint_live` | Listen twice on one endpoint | Second fails; first answers; token file unchanged | `tst_control_limits::secondServerRefusesALiveEndpoint` |
| `ac_endpoint_stale` | Bind a socket file with no lock holder, then listen | Succeeds | `tst_control_limits::staleEndpointIsReclaimed` |
| `ac_limit_new` | `document new` at 100000×100000 | `invalid_param`; count unchanged | `tst_control_limits::oversizedDocumentIsRefused` |
| `ac_limit_new_bytes` | `document new` at 30000×30000 | `invalid_param` from the byte limit | `tst_control_limits::documentOverTheByteLimitIsRefused` |
| `ac_limit_lasso` | `lasso_begin`, then 4097 `lasso_point` | 4097th is `invalid_param` | `tst_control_limits::lassoPointsAreCapped` |
| `ac_limit_name` | `document new` and `set_name` with 256 characters | Both `invalid_param` | `tst_control_limits::longNamesAreRefused` |
| `ac_limit_ui_tree` | `ui_tree` with `max_depth` 65, then `max_children` 1025 | Both `invalid_param` | `tst_control_limits::uiTreeLimitsAreEnforced` |
