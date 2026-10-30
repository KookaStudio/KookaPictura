## ADDED Requirements

### Requirement: Control requests run one at a time

The control server SHALL run requests one at a time, in arrival order, across all
connections, and SHALL NOT start a request while another is running, including
during a nested event loop. A connection SHALL hold at most 64 queued requests;
a request over that limit SHALL be answered `refused` without running. The
server SHALL accept at most 4 connections and SHALL close any further one. A
client that disconnects while its request runs SHALL NOT crash the application.

#### Scenario: Replies keep request order [ac_serial_order]

- **WHEN** a client writes several requests in one burst, the first of which
  opens a document
- **THEN** the responses arrive in request order and each sees the state left by
  the one before

#### Scenario: A queue over its limit is refused [ac_serial_queue_full]

- **WHEN** a client writes more than 64 requests while one is running
- **THEN** the excess requests are answered `refused` and the others run

#### Scenario: A fifth connection is closed [ac_serial_connections]

- **WHEN** four clients are connected and a fifth connects
- **THEN** the fifth is closed and the first four still get responses

#### Scenario: A disconnect during a request is safe [ac_serial_disconnect]

- **WHEN** a client disconnects while its request is running
- **THEN** the request finishes, no reply is written, and later clients are served

### Requirement: A dialog reached during a request is refused

The control server SHALL cancel any dialog or menu shown while a request runs and
SHALL answer that request `refused`, naming the dialog, so that no request waits
on a person. The progress window of a background job SHALL NOT be cancelled. A
dialog a person opens outside a request SHALL NOT be affected.

#### Scenario: An unlisted dialog does not hang the request [ac_guard_cancels]

- **WHEN** a client dispatches Image Size, which opens a dialog
- **THEN** the dialog is cancelled, the response is `refused`, and the document
  is unchanged

#### Scenario: Every command answers [ac_guard_every_command]

- **WHEN** every implemented and enabled command is dispatched in turn
- **THEN** each one gets a response and no dialog stays open

#### Scenario: A person's dialog is left alone [ac_guard_outside_request]

- **WHEN** a dialog is shown while no request is running
- **THEN** it stays open

### Requirement: Saving never reaches a dialog

`document save` SHALL refuse with `refused`, and SHALL NOT open a dialog, when the
active document would need the PDF or format dialog to save to its own path.

#### Scenario: A save needing a format dialog is refused [ac_save_no_dialog]

- **WHEN** a client calls `save` with no path on a layered document whose path
  cannot hold layers
- **THEN** the response is `refused`, no dialog opens, and the file is unchanged

### Requirement: A live control endpoint cannot be taken over

`listen()` SHALL fail, leaving the endpoint and its token untouched, while another
live process holds the endpoint. An endpoint left by a dead process SHALL be
reclaimed.

#### Scenario: A second server refuses a live endpoint [ac_endpoint_live]

- **WHEN** a second server listens on an endpoint a running server holds
- **THEN** `listen()` fails, the first server still answers, and its token is
  unchanged

#### Scenario: A stale endpoint is reclaimed [ac_endpoint_stale]

- **WHEN** a server listens on an endpoint whose socket file has no live holder
- **THEN** `listen()` succeeds

### Requirement: Control input is bounded

`document new`, and any resize a control request reaches, SHALL be refused with
`invalid_param` before allocating when the result exceeds `ImageBudget`. A
selection SHALL hold at most 4096 lasso points. A document or layer name SHALL
be at most 255 characters. `ui_tree` SHALL accept `max_depth` up to 64 and
`max_children` up to 1024. A value over a limit SHALL be `invalid_param`.

#### Scenario: An oversized document is refused [ac_limit_new]

- **WHEN** a client calls `document new` at 100000 by 100000
- **THEN** the response is `invalid_param`, the document count is unchanged, and
  the process stays alive

#### Scenario: A document at the edge of the budget is judged by bytes [ac_limit_new_bytes]

- **WHEN** a client calls `document new` within the dimension limit but over the
  allocation limit
- **THEN** the response is `invalid_param`

#### Scenario: Lasso points are capped [ac_limit_lasso]

- **WHEN** a client adds a 4097th lasso point
- **THEN** the response is `invalid_param`

#### Scenario: Long names are refused [ac_limit_name]

- **WHEN** a client names a document or a layer with 256 characters
- **THEN** the response is `invalid_param` and nothing changes

#### Scenario: ui_tree limits are enforced [ac_limit_ui_tree]

- **WHEN** a client calls `ui_tree` with `max_depth` 65 or `max_children` 1025
- **THEN** the response is `invalid_param`
