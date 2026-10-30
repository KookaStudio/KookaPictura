# Design: Harden the control server

## Serial pipeline

`onReadyRead` splits lines, appends `{socket, line}` to one queue, and queues a
call to `pump()`. The call is queued because a request can spin a nested event
loop, and a disconnect in there frees the socket; run from inside `readyRead`,
that frees a socket whose signal is still on the stack. `pump()` is guarded by
`running_`, so a nested event loop that delivers
another `readyRead` only enqueues. Sockets are held as `QPointer<QLocalSocket>`,
and a reply to a socket that has gone is dropped.

The queue is shared so a reply on one connection cannot overtake a mutation on
another. Limits: 64 queued lines per connection, 4 connections. A line over the
limit is answered `refused` straight away, without running; a fifth connection
is closed.

`document open` still waits in `openDocumentAtPathOutcome`, whose nested loop does
not exclude user input. `running_` keeps other requests out, and `ControlScope`
stays on across the wait because a PDF open can raise the import dialog.

ponytail: a dialog a person opens during a load in a visible `--control` window
is cancelled too. The live-session PR makes `open` deferred, which removes this.

## Modal guard

`ControlScope` is an RAII guard around `dispatch()`. It installs a
`qApp` event filter. On `QEvent::Show` of a `QDialog` or `QMenu` it records the
title and queues `reject()` / `close()`. The `jobBusy` progress dialog of
`runBusyJob` is exempt.

Why not `runDialog`: about 30 sites bypass it. Why not `ModalDialogWatcher`: it
needs `isModal()`, and `runDialog` sets `NonModal`.

After dispatch, a recorded dialog replaces the response with `refused`, even when
the handler reported success, because the command did not do what was asked.

ponytail: a native portal file dialog may never emit Show, so the guard can miss
it. The denylist still covers every known file-dialog command.

## Endpoint lock

`QLockFile` at `<endpoint>.lock` (in the temp dir on Windows, like the token) is
taken before `removeServer`. `QLockFile` reclaims a dead holder by PID, so a
crashed instance's socket stays reclaimable. `tryLock(0)` failing makes `listen()`
return false, so `--control` exits 153 without touching the live endpoint.

## Size budget

A new bridge `document_budget_allows(width, height, bits)` reads
`ImageBudget::default()`: each side within `max_dimension`, and bytes within
`max_alloc_bytes` and `max_document_bytes`. It is called in `document new`. Image
Size and Canvas Size have no dialog-free control route yet, so their check ships
with `run_command`, which opens that route.

## Split

`control_server.cpp` is 1056 lines. `control_server_documents.cpp` takes
`methodDocument`, `methodEdit`, `methodSetUnsavedPolicy`;
`control_server_capture.cpp` takes screenshot, `ui_tree`, thumbnail.
Shared helpers move to an internal `control_server_support.h`.
