# Design

## Context

- `PicturaMainWindow::showOpenDialog()` (`crates/pictura-app/cpp/frame.cpp`)
  reads one path from `getOpenFileName()` and starts one asynchronous open.
- `getOpenFileName()` (`cpp/dialogs.cpp`) builds the app-owned `QFileDialog`,
  sets `AcceptOpen`, and hard-codes `ExistingFile`.
- The drop router (`cpp/file_drop_router.cpp`) already opens every dropped path,
  one `startOpenDocumentAtPath()` per file, and refreshes once.
- `startOpenDocumentAtPath()` returns a token and decodes on a worker. Document
  entries are appended synchronously, so their order is the call order.

## Goals / Non-Goals

**Goals:**

- Shift-click a range and Ctrl/Cmd-click individual files in File > Open.
- One document per selected file, each with its own path.
- One unreadable selection cannot cancel the others.

**Non-Goals:**

- Changing Open As, Open As Smart Object, Place, Open Recent, or the command
  line.
- Serialising the opens or promising which document is active when they finish;
  the existing loading-tab lifecycle decides activation as each decode lands.
- Deduplicating or reordering the selection.

## Decisions

- **A separate `getOpenFileNames()` helper** rather than a parameter on
  `getOpenFileName()`: the other pickers must stay single-file, and the two
  functions share one dialog builder (`runOpenFileDialog`) so the Places
  sidebar, filter combo, and non-modal chrome stay identical.
- **The loop lives in `showOpenDialog()`** and starts every path before
  returning. It mirrors the drop router's explicit non-short-circuit loop; no
  wait between starts, so the loading tabs appear together and the workers run
  concurrently.
- **Host assignment is inherited.** `startOpenDocumentAtPath()` already routes
  through the loading tab and `hostReadyWindow()`, so the tabs preference needs
  no new branch.
- **Failure handling is inherited.** A failed decode removes only its own
  loading tab; the other tokens continue.

## Risks / Trade-offs

- Selecting several PDFs starts several PDF probes, so more than one Import PDF
  dialog can queue. The drop path already has this behaviour; a follow-up can
  serialise PDF imports if it becomes a problem.
- The active document after a multi-open is whichever decode finishes last,
  which is not deterministic for files of different sizes. The requirement does
  not name an active document.
