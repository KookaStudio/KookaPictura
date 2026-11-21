# Design

## Context

- The Rust catalogue (`crates/pictura-script/src/command_catalogue/`) is the
  single command table; `command_ids.h` is generated from it and the menu is
  built from catalogue rows (`crates/pictura-app/cpp/commands.cpp`).
- `File > Browse in Bridge` has no replacement row; Bridge is a declared
  non-goal (`docs/00-overview/feasibility-and-non-goals.md`), and the spec's
  replacement is `File > Browse…` opening the OS file dialog or the Files panel
  (`docs/10-workflow-io/bridge-and-interop.md`). The Files panel (#624) is not
  shipped.
- The Load Files into Stack pipeline already exists:
  `document_api::loadFilesIntoStack(view, paths, align, smartObject)` builds one
  document with the largest canvas, one layer per source, optional Auto-Align,
  and an optional embedded Smart Object. `set_smart_object_stack_mode(view,
  path, code)` is the shipped non-destructive stack-mode bridge.
- `SessionState` (`crates/pictura-app/cpp/session.h`) persists app-level dialog
  state; the Batch dialog already stores `batchOptions` there.
- `command_edition(id)` decides Extended-only rows; `addDefaultCommands` omits
  those rows in the Standard edition.
- Adobe's shipped `Statistics.jsx` and `CreateImageStack.jsx` define the CS6
  dialog: title **Image Statistics**, `Choose Stack Mode`, `Use: Files /
  Folder`, file list, `Browse…` / `Remove` / `Add Open Files`, the align
  checkbox, a two-file gate, Mean as the default mode, persisted mode/align,
  and a stack of at least two sources. Its menu label is `Statistics...`.
- See `proposal.md` for motivation and the spec deltas for the contracts.

## Goals / Non-Goals

**Goals:**
- Expose the Bridge replacement with no Bridge dependency and no new engine.
- Implement Statistics entirely by composing the shipped stack loader and
  stack-mode bridge.
- Keep menu data, edition gating, persistence, and docs consistent with the
  catalogue-first architecture.
- Leave a tested seam for the future scripting host without pretending to run
  scripts now.

**Non-Goals:**
- Adobe Bridge/Mini Bridge, the Files panel (#624), or any Bridge transport.
- A JavaScript/JSX host, QuickJS, script discovery, console, sandbox, or
  notifiers (#620/#621). `File > Scripts > Browse…` stays a disabled row.
- Changing the existing Load Layers dialog or its folder-mode contract.
- Reworking the stack-mode renderer, 16/32-bit stack rendering, or Auto-Align.
- Removing `Check In…`, `Import ▸ WIA Support`, or any other disabled row.

## Decisions

### D1. Browse is a file-manager hand-off, not a picker

`file.browse` opens the active document's containing folder, or the home folder
when no document has a path, through `QDesktopServices::openUrl` behind the
existing `urlOpener_` seam (`frame.h`, `setUrlOpenerForTest`). The handler is
registered with the other frame-owned File commands in `frame_commands.cpp`;
the shared URL helper moves to a private `PicturaMainWindow::openExternalUrl` so
the menu runtime and Browse use one path.

*Alternatives:* the Qt Open dialog (duplicates `File > Open` and gives no
browsing surface); the Files panel (unshipped, #624); a real Bridge launch
(non-goal, no Linux build). The spec explicitly allows the file manager.

### D2. Statistics composes the existing stack pipeline

The handler:
1. reads the remembered mode/align from `SessionState.statisticsOptions`
   (`{"mode": int, "align": bool}`, defaulting to Mean/off);
2. shows `StatisticsDialog` with the saved clean open-document paths, the
   unsaved-document flag, and the remembered values;
3. on accept, writes the accepted mode/align to
   `SessionState.statisticsOptions` before touching any document, mirroring
   `BatchHost::showDialog`'s save-before-run order;
4. calls `document_api::loadFilesIntoStack(view, paths, align,
   /*smartObject=*/true)`;
5. derives the loaded count as `paths.size() - failures.size()`; below two it
   deletes the view and warns, creating no document;
6. reads the top row from `layers_api::snapshot(view, 0, false)` and calls
   `set_smart_object_stack_mode(view, top, mode)`; a false return deletes the
   view and warns (the layers could not be combined);
7. adds the view as an untitled document and reports any skipped files.

No Rust bridge change is needed: the loader builds the stack and the stack-mode
bridge applies the reduction.

*Alternative:* extend `load_files_into_stack` with a mode parameter. Rejected:
it would change a shipped Rust API and its tests for a C++-side composition.

### D3. A separate Statistics dialog, CS6 source semantics

A new `StatisticsDialog` owns the controls; the existing `LoadLayersDialog` is
not refactored. Only the source-file suffix list and file-dialog filters move
from `load_layers_dialog.cpp` to `dialogs.h/.cpp`
(`stackSourceSuffixes()`, `stackSourceFilters()`) so both dialogs accept the
same formats.

The Statistics dialog follows the CS6 shared-dialog code, not the app's Load
Layers folder mode: `Use: Folder` changes what `Browse…` chooses, the chosen
folder's supported images are **added to the list**, and `Remove` / `Add Open
Files` stay enabled. The list is the source of truth; switching `Use` does not
clear it.

*Alternative:* reuse or generalize `LoadLayersDialog`. Rejected: it would add
mode/row knobs and change a shipped dialog's folder behavior for one new user.

### D4. Stack-mode order, default, and persistence

The mode menu uses Adobe's `Statistics.jsx` order and the app's existing codes:

| Label | Code |
|---|---|
| Mean | 3 |
| Summation | 5 |
| Minimum | 2 |
| Maximum | 1 |
| Median | 4 |
| Variance | 7 |
| Standard Deviation | 8 |
| Skewness | 9 |
| Kurtosis | 10 |
| Range | 6 |
| Entropy | 11 |

Mean is the default because the script selects its first item; the "Median is
default" advice applies to later Photoshop versions. On accept the handler
writes `SessionState.statisticsOptions` with the numeric code and align flag
before loading the stack; the option reader falls back to Mean for an unknown
code. The script stores the same two values in
`app.putCustomOptions('ImageStatistics', …)`.

*Alternative:* no persistence. Rejected: it is verified CS6 behavior and the
session store already carries dialog state.

### D5. Catalogue changes and edition gate

- Insert `file.browse` after `file.open` (the real CS6 position; the current
  `docs/02-ui-ux/menus.md` order is corrected in this change).
- Flip `file.scripts.statistics` implemented and label it `Statistics…`
  (Adobe's menu resource string).
- Add `file.scripts.browse` (`File > Scripts > Browse…`) with
  `implemented: false`; it has no handler and stays disabled until the scripting
  host flips it on. The command-registry requirement already covers disabled
  rows.
- Add `file.scripts.statistics` to the Extended edition list and its catalogue
  test; the menu skips it in Standard.
- Regenerate `command_ids.h`; add `FileBrowse` and `FileScriptsStatistics`
  constants; update both goldens.

### D6. Failure and two-source rules

The dialog's `OK` and align checkbox enable at two files, matching the script's
`mergeFiles.length > 1`. At execution, unreadable sources are skipped; if fewer
than two decode, no document is created. This mirrors `loadStackLayers()`
closing the stack when fewer than two usable elements remain.

### D7. Test seams

- Browse: `setUrlOpenerForTest` captures the URL instead of launching the
  desktop file manager.
- Statistics merge: the command is driven with the existing modal-dialog test
  pattern (timer + `QApplication::topLevelWidgets`). `StatisticsDialog` exposes
  `StatisticsDialog(const QStringList& openPaths, bool hasUnsavedOpenDocuments,
  int initialMode, bool initialAlign, const QString& startFolder,
  QWidget* parent = nullptr)`, `paths()`, `alignImages()`, `stackModeCode()`,
  `addFiles()`, `addFolder()`, and `addOpenFiles()` for tests; the remaining
  controls are exercised through their object names. Pixels are read with
  `PictureView::composite_argb`.
- Persistence: read `pictura::loadSession().statisticsOptions`.
- Native file/folder pickers: manual (no file-dialog driver exists).

## Risks / Trade-offs

- **The Adobe script mirror is the 2006-shipped `Statistics.jsx`, not the CS6
  binary.** The CS6 Help and tutorials agree on the workflow, title, controls,
  and two-file rule; the mirror supplies the mode order, default, and persisted
  choice. → Cite both in the change; keep the spec to observable behavior.
- **`set_smart_object_stack_mode` records a second history state** after the
  stack document is built. → Acceptable: the script also performs stack
  creation and rendering as separate operations; no spec promises one undo
  boundary.
- **Auto-Align on synthetic fixtures may be weaker than on real photos.** →
  The align scenario uses a translated feature; if the alignment engine cannot
  register the fixture, keep the dialog pass-through test and mark that
  scenario manual with the reason.
- **The stack engine narrows 16/32-bit sources to its 8-bit render channels.**
  → Already documented by `compositing/smart-object-stack-modes`; the
  Statistics spec does not promise native-depth reduction.
- **File-manager behavior is desktop-dependent.** → Only the handed-off URL is
  contractual; the OS launch itself is not asserted.
- **A disabled Scripts `<Browse…>` row is a dead menu entry.** → Owned by #620;
  the register records that dependency, and the row matches the repo's existing
  disabled-stub convention.
- **Session persistence adds a new key.** → Missing or invalid values default to
  Mean/off, so older session files load unchanged.

## Migration Plan

No data or format migration. Ship the catalogue, dialog, handlers, and tests;
archive the change in the same pull request; update the parity register UI-8
and AUTO-4, the menu-tree doc, `ROADMAP.md`, and `docs/dev/STATE.md`.
