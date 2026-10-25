# agentic-control Specification

## Purpose
An opt-in local control mode exposing a newline-delimited JSON protocol for non-interactive state readback and actions.

## Requirements

### Requirement: Opt-in control mode over a local socket

The application SHALL start a control server only when invoked with `--control`.
The server SHALL listen on a per-user Unix-domain socket whose default path is
`$XDG_RUNTIME_DIR/pictura-control.sock`, overridable with `--control-socket
PATH`; the resolved path SHALL be printed to stderr. `--state-home DIR` SHALL
isolate the session store to `DIR`. `--headless --control` SHALL run offscreen
without a display. Without `--control`, no socket SHALL be created and startup,
the self-test, and normal runs SHALL be unchanged.

#### Scenario: Control mode starts and reports its socket

- **WHEN** the app is launched with `--control`
- **THEN** it creates the socket, prints `pictura control: <path>` to stderr, and
  a client that connects receives a `status` response

#### Scenario: No control server without the flag

- **WHEN** the app is launched without `--control`
- **THEN** no socket path is printed and the default socket is not created

#### Scenario: A custom socket path is honored

- **WHEN** the app is launched with `--control --control-socket /tmp/x.sock`
- **THEN** the server listens on `/tmp/x.sock`

### Requirement: Newline-delimited JSON protocol

The server SHALL read one JSON object per line. A request SHALL be
`{"id":<int>,"method":<string>,"params":<object>}`. A success response SHALL be
`{"id":<int>,"ok":true,"result":<object>}`; a failure response SHALL be
`{"id":<int>,"ok":false,"error":{"code":<string>,"message":<string>}}` and SHALL
echo the request's integer `id` whenever the request carried one (an unparseable
line has no id to echo). JSON SHALL be serialized compact so a message never
contains a raw newline. A malformed or unparseable line SHALL produce a
`bad_request` error and SHALL NOT crash the application. The error codes SHALL be
`bad_request`, `unknown_method`, `invalid_param`, `no_document`,
`not_implemented`, `refused`, `io_error`, and `internal`.

#### Scenario: A malformed line does not crash the app

- **WHEN** a client sends a line that is not valid JSON
- **THEN** the server responds with an error whose code is `bad_request` and the
  process stays alive

#### Scenario: An unknown method is reported

- **WHEN** a client sends `{"id":7,"method":"no_such_method","params":{}}`
- **THEN** the response has `id` 7, `ok` false, and error code `unknown_method`

#### Scenario: A document method with no document is reported

- **WHEN** a document-requiring method is invoked while no document is open
- **THEN** the response has error code `no_document`

### Requirement: Non-interactive control mode

Control mode SHALL set the unsaved-prompt policy to non-interactive with the
Discard choice at startup. The `document` method SHALL use the non-dialog entry
points and SHALL NOT reach `showOpenDialog`, `showNewDocumentDialog`, or any
other modal; in particular a `save` with no path on a document without a file
path SHALL return `invalid_param` rather than opening Save-As. Because
`dispatch_command` invokes the real menu handlers, some of which open modal
dialogs, it SHALL refuse a command that opens a modal dialog (or that can reach
the unsaved prompt) with the `refused` error and SHALL NOT invoke its handler.
Consequently, no control request SHALL block on a modal dialog.

#### Scenario: Opening a document never opens a file dialog

- **WHEN** a client invokes `document` with op `open` and a path
- **THEN** the document opens without a modal dialog and the response reports the
  new active index and count

#### Scenario: A dirty-document close does not prompt

- **WHEN** a document is dirty and a client invokes `document` with op `close`
- **THEN** the document closes without a modal, per the non-interactive Discard
  policy

#### Scenario: Saving an untitled document is refused

- **WHEN** a client invokes `document` with op `save` on a document with no file
  path
- **THEN** the response is `invalid_param` and no Save dialog opens

#### Scenario: A modal-opening command is refused

- **WHEN** a client invokes `dispatch_command` with a command that opens a modal
  dialog
- **THEN** the response is `refused` and the handler does not run

### Requirement: State and inspection readback

The server SHALL provide these read-only methods returning a JSON object:
`status` (document count, active index, per-document name/path/width/height/
mode/depth/dirty/layer-count/selection-count, active tool, backend,
gpu-available, brightness, screen mode, panel names/visibility, zoom);
`get_pixel` (`x`, `y` → the active document's composited `0xAARRGGBB` plus the
channel values); `list_layers` (per layer index/name/kind/visible/opacity/blend/
fill/lock/color); and `list_commands` (per command id/path/label/implemented/
enabled/checked, optionally restricted to implemented commands).

#### Scenario: Status describes the active document

- **WHEN** a client invokes `status` with one open document
- **THEN** the result reports one document, its dimensions, mode, depth, and
  layer and selection counts

#### Scenario: A pixel can be read back

- **WHEN** a client invokes `get_pixel` at a coordinate inside the document
- **THEN** the result carries the composited ARGB and channel values at that
  coordinate

#### Scenario: Layers and commands are enumerable

- **WHEN** a client invokes `list_layers` and `list_commands`
- **THEN** the results enumerate the active document's layers and the command
  registry with each command's implemented/enabled/checked state

### Requirement: Non-interactive action surface

The server SHALL provide `dispatch_command` (`id` → whether it dispatched and
its resulting enabled state, then a refresh; a modal-opening command is refused
as above), `document` (op `open`/`new`/`save`/`save_as`/`revert`/`close`/
`activate`/`list` with the op-specific parameters; `open` accepts both a native
PSD/PSB path and a raster image path, matching the GUI), `edit` (op
`undo`/`redo` → `success` plus can-undo/can-redo/history depth), and
`set_unsaved_policy` (`interactive` and optional `choice` = `cancel`/`discard` →
the resulting policy). A mutation SHALL be applied through the same paths the
application uses for its own commands. When `set_unsaved_policy` has re-armed
the interactive prompt, `dispatch_command` SHALL refuse the commands that can
reach that prompt.

#### Scenario: Dispatching a command performs its action

- **WHEN** a client invokes `dispatch_command` with an implemented command id
- **THEN** the command's action runs and the response reports it dispatched

#### Scenario: Undo and redo round-trip

- **WHEN** a mutation is made and a client invokes `edit` with op `undo` then
  `redo`
- **THEN** each reports success and the history depth reflects the change

#### Scenario: The unsaved policy can be flipped

- **WHEN** a client invokes `set_unsaved_policy` with `interactive` true and
  choice `cancel`
- **THEN** the response reports the new interactive state and choice

### Requirement: Local-only security

The socket SHALL be a Unix-domain socket created with user-only access (owner
read/write and no group or other access; Qt's `UserAccessOption` creates it with
mode `0700`), SHALL NOT be a TCP listener, and the control surface SHALL NOT
include any arbitrary-code-evaluation method. The control server SHALL exist
only under `--control`. A line or buffered request larger than a fixed bound
SHALL be rejected with `bad_request` rather than growing without limit.

#### Scenario: The socket is user-only

- **WHEN** the control server creates its socket
- **THEN** the socket's permissions grant access to the owning user only

#### Scenario: No eval method exists

- **WHEN** a client attempts a method that would evaluate arbitrary code
- **THEN** the response is `unknown_method`

#### Scenario: An oversized request is rejected

- **WHEN** a client sends an unterminated request that exceeds the fixed limit
- **THEN** the server responds with `bad_request` and does not grow the buffer
  without bound

### Requirement: Verification of the control server

The self-test SHALL include a `mcp_control` block that starts a control server on
a temporary socket, connects a client, and round-trips at least `status`,
`list_commands`, and a `dispatch_command`, asserting the response shapes and the
`bad_request`/`unknown_method` error paths, using the next append-only check
code.

#### Scenario: The control block proves the socket and core methods

- **WHEN** the self-test runs the `mcp_control` block
- **THEN** a client connects to the temporary socket and receives well-formed
  `status`, `list_commands`, and `dispatch_command` responses, and a malformed
  line yields `bad_request`

### Requirement: Screenshot readback

The server SHALL provide a `screenshot` method with a `scope` of `window`,
`canvas`, or `document` (default `window`) and an optional `max_dim` (default
1280). It SHALL return `{mime:"image/png", base64, width, height, source_width,
source_height}`: the PNG of the whole frame (`window`), the active canvas widget
including its checkerboard/zoom/overlay (`canvas`), or the raw composited
document image (`document`). When the image's long edge exceeds `max_dim` it
SHALL be scaled down preserving aspect ratio, and the pre-scale dimensions SHALL
be reported as `source_width`/`source_height`; `width`/`height` are the returned
image's. `canvas` and `document` with no active document SHALL return
`no_document`. A non-positive `max_dim` SHALL be `invalid_param`.

#### Scenario: A window screenshot is a PNG

- **WHEN** a client invokes `screenshot` with `scope` `window`
- **THEN** the result's `mime` is `image/png`, its `base64` decodes to a PNG, and `width`/`height` match the decoded image

#### Scenario: A canvas screenshot with no document is reported

- **WHEN** a client invokes `screenshot` with `scope` `canvas` and no document is open
- **THEN** the response has error code `no_document`

#### Scenario: max_dim downscales and reports the source size

- **WHEN** a client invokes `screenshot` with a `max_dim` smaller than the source long edge
- **THEN** the returned image's long edge is at most `max_dim`, `width`/`height` are the returned image's dimensions, and `source_width`/`source_height` are the original ones

#### Scenario: A non-positive max_dim is rejected

- **WHEN** a client invokes `screenshot` with `max_dim` 0 or negative
- **THEN** the response has error code `invalid_param`

### Requirement: UI tree readback

The server SHALL provide a `ui_tree` method with optional `max_depth` (default
12) and `max_children` (default 64) limits. It SHALL return a depth- and
breadth-capped array of nodes, each `{class, objectName, rect:{x,y,w,h},
visible, enabled, text, tooltip}`, where `rect` is in window (frame) coordinates.
A default call SHALL include the layers panel's object name so an agent can
locate it (the panel sits ~9 widget levels below the frame, well within the
default depth).

#### Scenario: The default tree contains the layers panel

- **WHEN** a client invokes `ui_tree` with no parameters
- **THEN** some node's `objectName` is `layersPanel` and its `rect` is a window-local rectangle

#### Scenario: The walk is bounded

- **WHEN** a client invokes `ui_tree` with small `max_depth`/`max_children`
- **THEN** the returned tree does not exceed those bounds and the server responds without error

### Requirement: Layer thumbnail readback

The server SHALL provide a `layer_thumbnail` method with `index` and an optional
`size` (default 64). For a layer that has a thumbnail it SHALL return the same
image shape as `screenshot` (`mime`, `base64`, `width`, `height`,
`source_width`, `source_height`); for a group, an adjustment layer, or an
out-of-range index it SHALL return `invalid_param`.

#### Scenario: A pixel layer has a thumbnail

- **WHEN** a client invokes `layer_thumbnail` for a pixel layer
- **THEN** the result's `mime` is `image/png` and its `base64` decodes to a PNG of at most `size` on the long edge

#### Scenario: A group layer has no thumbnail

- **WHEN** a client invokes `layer_thumbnail` for a group layer
- **THEN** the response has error code `invalid_param`

### Requirement: Vision verification

The self-test SHALL include a block that exercises the three vision methods over
the control socket, using the next append-only check codes. It SHALL assert that
a `screenshot` is a PNG and that its reported dimensions match the decoded image,
that a downscaled `screenshot` reports the source dimensions, that `ui_tree`
contains the layers panel, and that a pixel layer's `layer_thumbnail` is a PNG
while a group's is `invalid_param`.

#### Scenario: The vision block proves the methods offscreen

- **WHEN** the self-test runs the vision block under `--headless`
- **THEN** every vision assertion passes and the block reports pass, using no display

### Requirement: Tool selection

The server SHALL provide a `set_tool` method with a `tool` name. It SHALL accept
either the name `status` reports as `active_tool` or the tool's display label,
set the active tool through the application's own path, and return
`{active_tool}`. An unknown name SHALL be `invalid_param`.

#### Scenario: A tool can be selected by its status name

- **WHEN** a client invokes `set_tool` with the name `status` reported for the active tool or another known tool
- **THEN** the response reports that tool as `active_tool` and a later `status` agrees

#### Scenario: An unknown tool is rejected

- **WHEN** a client invokes `set_tool` with an unknown name
- **THEN** the response has error code `invalid_param`

### Requirement: Selection actions

The server SHALL provide a `selection` method with an `op` of `all`, `deselect`,
`rect`, `ellipse`, `lasso_begin`, `lasso_point`, `lasso_end`, `quick`, or
`wand`, taking the op's coordinates, `mode` (`new`/`add`/`subtract`/`intersect`),
`tolerance`, `contiguous`, and `feather`. `mode` SHALL be one of those four
values and any other value SHALL be `invalid_param` (an unknown mode must not
silently behave as `new`). It SHALL drive the selection model through the
application's paths and return `{has_selection, count, bounds}` where `bounds` is
the selection's `x y w h` when one exists. An op the selection model does not
apply (a rectangle or ellipse with non-positive `w`/`h`, a lasso with fewer than
three points, a wand outside the document) SHALL return `invalid_param`.

#### Scenario: A rectangular selection is applied

- **WHEN** a client invokes `selection` with op `rect` and a rectangle inside the document
- **THEN** the result reports `has_selection` true and a `count` greater than zero

#### Scenario: Deselect clears the selection

- **WHEN** a client invokes `selection` with op `deselect` while a selection exists
- **THEN** the result reports `has_selection` false and `count` zero

#### Scenario: A selection needs a document

- **WHEN** a client invokes `selection` with no document open
- **THEN** the response has error code `no_document`

#### Scenario: A non-applying selection is rejected

- **WHEN** a client invokes `selection` with op `rect` and `w` of zero, or with `mode` `union`
- **THEN** the response has error code `invalid_param`

### Requirement: Filter and adjustment actions

The server SHALL provide a `filter` method (`kind`) that applies one of the
application's filter kinds and returns `{ok, kind}`, and an `adjustment` method
(`kind`) that adds an adjustment layer of that kind and returns `{ok, layers}`.
An unknown kind SHALL be `invalid_param`. A `filter` whose target layer is
unavailable (no editable pixel layer), hidden, or pixel-locked SHALL return
`refused`, so an engine refusal is never misreported as an unknown kind. The
application's fixed seeds make a seeded filter deterministic.

#### Scenario: A seeded filter is deterministic

- **WHEN** a client applies the same seeded `filter` kind twice to the same document
- **THEN** both calls succeed and produce the same pixels

#### Scenario: An unknown kind is rejected

- **WHEN** a client invokes `filter` or `adjustment` with an unknown kind
- **THEN** the response has error code `invalid_param`

#### Scenario: A locked target refuses the filter

- **WHEN** a client pixel-locks the active layer and invokes `filter`
- **THEN** the response has error code `refused` and the kind is not reported as unknown

### Requirement: Layer operations

The server SHALL provide a `layer_op` method with an `op` of `set_name`,
`set_opacity`, `set_visible`, `set_blend`, `set_fill`, `set_lock`, `set_color`,
`move`, `translate`, `delete`, `duplicate`, or `add`, taking `index` and the op's
value. It SHALL apply the operation through the application's layer methods and
return `{ok, layers}`. An out-of-range index or a refused operation SHALL be
reported with the application's error (`invalid_param`/`refused`), not a crash.
`set_visible` SHALL require an explicit boolean, supplied as either `visible` or
`on`, and SHALL return `invalid_param` when neither is a boolean; a missing or
non-boolean value SHALL NOT default to visible. `set_name` SHALL require a
non-empty `name`. `set_lock` SHALL take a `flag` of `transparency`, `pixels`,
`position`, `nesting`, or `all` with an `on` boolean (`list_layers` reports
`lock` as the resulting bitmask). `translate` targets the active layer; when
`index` is present the server SHALL make that layer active before translating so
the requested layer is the one moved, and an out-of-range `index` SHALL be
`invalid_param`.

#### Scenario: A layer property round-trips

- **WHEN** a client sets a layer's opacity with `layer_op` and then reads it with `list_layers`
- **THEN** the reported opacity is the value that was set

#### Scenario: Visibility must be explicit

- **WHEN** a client invokes `layer_op` with op `set_visible` and `on` false
- **THEN** `list_layers` reports the layer as not visible

#### Scenario: A duplicate is created

- **WHEN** a client invokes `layer_op` with op `duplicate`
- **THEN** the result reports one more layer than before

#### Scenario: An out-of-range index is reported

- **WHEN** a client invokes `layer_op` with an out-of-range index
- **THEN** the response has error code `invalid_param` and the application does not crash

#### Scenario: Translate honors its index

- **WHEN** a client invokes `layer_op` with op `translate` and an in-range `index`
- **THEN** that layer is made active and moved; an out-of-range `index` returns `invalid_param`

### Requirement: GPU compute toggle

The server SHALL provide a `set_gpu_compute` method with an `on` boolean that
sets the compute preference through the application and returns
`{gpu_compute, backend}`.

#### Scenario: The toggle is reported

- **WHEN** a client invokes `set_gpu_compute` with `on` false
- **THEN** the result reports `gpu_compute` false and the active backend

### Requirement: Action verification

The self-test SHALL include a block that exercises the action methods over the
control socket using the next append-only check codes: it SHALL select a tool,
apply a rectangular `selection` and assert a non-zero count, apply a seeded
`filter` twice and assert determinism plus an `invalid_param` for an unknown
kind, round-trip a `layer_op` opacity through `list_layers`, and toggle
`set_gpu_compute`.

#### Scenario: The action block proves the methods offscreen

- **WHEN** the self-test runs the action block under `--headless`
- **THEN** every action assertion passes and the block reports pass

### Requirement: Pointer input synthesis

The server SHALL provide a `pointer` method with an `op` of `click`, `dblclick`,
`move`, `drag`, or `scroll`, coordinates `x`/`y` (and `x2`/`y2` for `drag`), a
`space` of `window` or `image`, and optional `button`, `modifiers`, and `steps`.
It SHALL synthesize the corresponding `QMouseEvent`/`QWheelEvent` and deliver
them with `QApplication::sendEvent` through the application's real event path; it
SHALL NOT link or call a test framework. With `space` `image` the coordinates are
document pixels on the active `ImageView`, mapped by `widget = image * zoom +
offset`; with no active document it SHALL return `no_document`. With `space`
`window` the coordinates are frame-local (the space `ui_tree` reports). An
unknown `op` or `space`, a non-numeric coordinate, a `drag` without `x2`/`y2`, or
a `steps` outside its bound SHALL be `invalid_param`. It SHALL return `{ok}`.

#### Scenario: A drag in image space drives a tool

- **WHEN** a client selects a marquee tool and invokes `pointer` with op `drag`, space `image`, and a rectangle inside the document
- **THEN** the result is `ok` and a selection exists whose bounds cover the dragged rectangle

#### Scenario: Image space needs a document

- **WHEN** a client invokes `pointer` with space `image` and no document open
- **THEN** the response has error code `no_document`

#### Scenario: A malformed pointer request is rejected

- **WHEN** a client invokes `pointer` with an unknown `op`, or op `drag` without `x2`/`y2`
- **THEN** the response has error code `invalid_param`

### Requirement: Key input synthesis

The server SHALL provide a `key` method taking a `sequence` such as `Ctrl+Z`,
`B`, or `Shift+F2`. It SHALL parse the sequence into a modifier set and a key,
synthesize a `QKeyEvent` press and release, and deliver them with
`QApplication::sendEvent` to `QApplication::focusWidget()` when one exists, else
the frame. A synthesized (non-spontaneous) key press SHALL be routed through the
application's shortcut machinery, so an installed `QAction`/`QShortcut` whose
shortcut matches is triggered first, and a key that no installed shortcut
consumes reaches the widget's `keyPressEvent`. (A window-scoped shortcut matches
only while its window is active; the in-process self-test runs before the window
activates, so it exercises the widget-handler path, and a live run exercises the
shortcut path.) An unknown key name or modifier SHALL be `invalid_param`. It
SHALL return `{ok}`.

#### Scenario: An installed shortcut fires

- **WHEN** a client invokes `key` with a sequence bound to an installed shortcut in an active window (for example the screen-mode/brightness shortcut)
- **THEN** the result is `ok` and the shortcut's effect is observable in a later readback

#### Scenario: A key with no shortcut reaches the widget handler

- **WHEN** a client invokes `key` with a key that no installed shortcut consumes (for example an arrow key while the Move tool is active)
- **THEN** the result is `ok` and the widget's key-handler effect is observable in a later readback

#### Scenario: An unknown key is rejected

- **WHEN** a client invokes `key` with a sequence naming an unknown key
- **THEN** the response has error code `invalid_param`

### Requirement: Input verification

The in-process self-test SHALL drive a `pointer` drag and a `key` sequence
through the control server and assert the observable effect, so input synthesis
is checked offscreen without a display.

#### Scenario: The self-test drags a marquee

- **WHEN** the control self-test opens the fixture, selects the marquee tool, and sends a `pointer` drag in image space
- **THEN** it reports a committed selection with a non-zero pixel count and passes

### Requirement: Tool options readback and patch

The server SHALL provide a `tool_options` method. Given an `options` object it
SHALL merge it into the tool options exactly as the session store restores
them (a group or field it leaves out keeps its value, each value is clamped as
its setter would), and with or without one it SHALL return `{options}`, every
tool's options as the session store writes them. An `options` that is not an
object SHALL be `invalid_param`.

#### Scenario: A brush size is patched and read back [ctl_tool_options]

- **WHEN** a client sends `tool_options` with `{"brush": {"size": 321}}`, then
  without options, then with `options` set to a number
- **THEN** the first response reports brush size 321 and the Brush uses it, the
  second reports the options, and the third is `invalid_param`

### Requirement: Held-button pointer steps

The `pointer` method SHALL also accept `op` `down` and `up`, a button press and
release at `x`/`y`. Between a `down` and its `up`, a `move` SHALL carry the
pressed button, so a client can send each motion of a drag as its own request
and time it; any other `move` carries no button. A `down` while a button is
already held SHALL be `invalid_param`, and a `click`, `dblclick` or `drag`
SHALL leave no button held.

#### Scenario: A drag sent a step at a time paints [ctl_pointer_steps]

- **WHEN** a client selects the Brush, sends `pointer` `down`, several `move`s,
  and `up` in image space, then one more `move`
- **THEN** one stroke is recorded along the moves, and the last `move` paints
  nothing

#### Scenario: A held button cannot go stale [ctl_pointer_stale]

- **WHEN** a client sends `down`, then a second `down`, then a `click`, then a
  plain `move`
- **THEN** the second `down` is `invalid_param`, and after the `click` the
  `move` paints nothing

### Requirement: Control requests carry a session token

On listening, the control server SHALL write a fresh random session token
(64 hex digits) to `<socket>.token`, readable and writable by the owner only,
created fresh without following an existing link, and removed on shutdown.
Every socket request SHALL carry the token as its `token` member; a
well-formed request without it, or with any other value, SHALL be answered
`unauthorized` (echoing its `id`) and not dispatched.

#### Scenario: A request without the token is refused [ac_token_required]

- **WHEN** a client sends `status` with no token, a wrong token of the same
  length, or a short one, and then with the session token
- **THEN** the first three are answered `unauthorized` with their `id`, and
  the last succeeds

#### Scenario: The token file is private [ac_token_file]

- **WHEN** the server listens where a symlink to another file was planted at
  the token path, and later shuts down
- **THEN** the token file is a regular owner-only file holding the token, the
  link's target is untouched, a second server's token differs, and shutdown
  removes the file

### Requirement: Metrics readback

The server SHALL provide a `metrics` method. When the application runs with
metrics on (`verification/performance-metrics`), the method SHALL return
`{enabled: true, phases, counters, edits}`:

- `phases`: maps each phase label to `{count, total_ms, max_ms}`;
- `counters`: holds the GPU resident hits and misses and the decline counts by
  reason;
- `edits`: lists the most recent edit records, newest last.

With `reset: true`, the method SHALL clear the phase totals and the edit records
after reading them. When metrics are off, the method SHALL return the error code
`refused`.

#### Scenario: Snapshot after an edit

- **WHEN** metrics are on and a client calls `metrics` after one New Layer
- **THEN** the response contains a `New Layer` edit record and non-empty phases

#### Scenario: Reset clears the totals

- **WHEN** a client calls `metrics` with `reset: true` and then calls it again
  without editing
- **THEN** the second response has no phases and no edit records

#### Scenario: Disabled metrics are refused

- **WHEN** metrics are off and a client calls `metrics`
- **THEN** the response is an error with code `refused`

### Requirement: Group creation over layer_op

`layer_op` SHALL accept op `add_group`, which creates an empty group above
`index` through the same application method as the Layers panel's New Group
button. It SHALL return `{ok, layers}`. An out-of-range `index` SHALL return
`invalid_param`.

#### Scenario: A group is added

- **WHEN** a client invokes `layer_op` with op `add_group` and an in-range index
- **THEN** the result reports one more top-level node, and that node is a group
