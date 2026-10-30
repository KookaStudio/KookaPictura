# Proposal: Harden the control server for agent use

## Why

#1053 puts the control server behind an MCP bridge, so agents will send it
requests a test script never would. Reading the server found gaps that break
its own "no control request blocks on a modal" rule, or let a request take the
app down (#1061):

- The `dispatch_command` denylist misses Image Size, Fill, Stroke and more, so
  those requests never get a reply.
- A pathless `save` can reach the PDF and format dialogs.
- `document open` and busy saves spin nested event loops, so a second request
  can run mid-operation and replies can arrive out of order. A disconnect in
  that window can free a socket the handler still holds.
- A second instance unlinks a live socket and replaces its token.
- `document new`, Image Size and Canvas Size have no size cap, and an
  allocation failure aborts the process.
- Lasso points, names, connections and `ui_tree` limits are unbounded.

## What changes

- Requests are queued and run one at a time, in order, across every connection.
  Queues and connections are bounded.
- Any dialog or menu reached during a request is cancelled and the request is
  answered `refused`. The denylist stays as the first check, so a listed
  command's handler still never runs.
- `save` refuses, rather than reaches, a document that needs a dialog.
- A per-endpoint lock stops a second instance taking over a live endpoint, and
  a stale endpoint from a dead process is still reclaimed.
- `document new` is checked against `ImageBudget` before allocating. Input caps
  cover lasso points, names and `ui_tree`. Resizes get the same check when a
  dialog-free control route to them ships.
- Pure-move split of `control_server.cpp` along its method groups.

Live sessions, permissions and new methods are out of scope (next PRs of #1053).

## Capabilities

- Modified (added requirements): `interop/agentic-control`.

## Impact

- `pictura-app`: `control_server*.cpp`, `control_scope.{h,cpp}`, the size-budget
  bridge; new Qt suites `tst_control_serial`, `tst_control_modal_guard`,
  `tst_control_limits`.
- `CMakeLists.txt` and the tests list gain the new files.
- No new dependencies.
