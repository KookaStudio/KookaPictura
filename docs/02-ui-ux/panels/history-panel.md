# History Panel

- **Spec ID:** `PAN-004`
- **Status:** `Draft`
- **Parity tier:** `Core`.
- **New in CS6:** `No` — the History panel is CS5-era; CS6 does not change its model. (The panel is documented alongside the CS6 Help's general undo workflow; the JDI/What's-New list does not flag a History-panel change.)
- **Depends on:** `ARCH-009` undo-history, `TOOL-033` history-brush, `TOOL-034` art-history-brush, `TOOL-023` eraser-tools (Erase to History), `LAY-011` layer-styles (layer-visibility undo option), `ARCH-008` document-model, `PAN-001` layers-panel, `10-workflow-io/file-info-and-metadata.md` (Edit History Log).

> Crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. The history-state engine, memory strategy and
> undo-record shape are owned by `ARCH-009`; this file specifies the **panel
> widget** and the user-visible history behaviors.

## CS6 behavior

The **History panel** (`Window > History`, or its tab) records every change made to the image in the current working session, so the image can be jumped to any recent state. Selecting a state reverts the image to how it looked when that change was first applied; the user can then work from that state. It also deletes states, creates a document from a state or snapshot, and selects the source for the History Brush.

**Panel anatomy (CS6 Help figure).** A. Sets the source for the history brush; B. thumbnail of a snapshot; C. history state; D. history state slider. Snapshots are listed above the states; the bottom button strip holds **Create New Snapshot**, **Create a New Document From Current State**, and the **Delete** icon. **Default placement:** in the default Essentials workspace the History panel sits in the narrower secondary column, collapsed to its icon, above **Properties**; it is also part of the Painting workspace (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

### States

- Each applied change adds a **state**, listed with the name of the tool or command used.
- By default the panel lists the previous **20 states**; the number is set in `Edit > Preferences > Performance > History States` (`1`–`1,000`, default `20`, bounded by scratch space). Older states are automatically deleted to free memory; to keep a particular state for the whole session, make a snapshot of it.
- States are added at the bottom: oldest at top, most recent at bottom.
- Selecting a state dims the states below it, showing what would be discarded.
- By default, changing the image after selecting a state eliminates (deletes) all states after it. That change can be undone with `Undo`, restoring the eliminated states.
- States and snapshots are cleared when the document is closed and reopened.
- Program-wide changes (panels, color settings, actions, preferences) are **not** recorded, because they are not image changes.
- `File > Revert` is added as a history state and can be undone.

### Snapshots

A **snapshot** is a named temporary copy of any state, listed at the top of the panel. Snapshots can be named, persist for the whole work session, let the user compare effects, and provide a recovery point. They are **not saved with the image** — closing the image deletes them. Unless `Allow Non-Linear History` is on, selecting a snapshot and changing the image deletes all currently listed states.

- Create via the `Create New Snapshot` button (automatic), `New Snapshot` from the panel menu when `Automatically Create New Snapshot When Saving` is on, or `Alt`/`Option`-click the button to set options.
- Snapshot **From**: `Full Document` (all layers), `Merged Layers`, `Current Layer`.
- Rename by double-clicking; delete via the panel menu, the Delete icon, or dragging to it.

### Restore, delete, purge, new document

- **Restore part of an image:** History Brush (`TOOL-033`), Eraser with **Erase To History** (`TOOL-023`), or `Edit > Fill > Use: History`. To restore using the initial state, `History Options > Automatically Create First Snapshot` must be on.
- **Delete a state:** select it and choose `Delete` (deletes it and all later states) or drag it to the Delete icon. With `Allow Non-Linear History`, deleting a state deletes only that state.
- **Clear History** removes the state list without changing the image and does **not** reduce memory. `Alt`/`Option` `Clear History` purges the list from the Undo buffer and **frees memory**, and cannot be undone. `Edit > Purge > Histories` purges all open documents' states and cannot be undone.
- **New document from a state/snapshot:** drag the state/snapshot onto the `Create a New Document From Current State` button, select it and click the button, or choose `New Document` from the panel menu. The new document's history list contains only the `Duplicate State` entry. A state can also be dragged onto an existing document.
- **Saving snapshots across sessions:** save each state/snapshot as its own file, then reopen the original and drag each file's initial snapshot back — snapshots are runtime-only.

### History brush source

Clicking the **left column** of a state or snapshot sets it as the History Brush source. Unless a merged snapshot is selected, the brush copies from that layer in the selected state to the same layer in another state, at the same location (`TOOL-033`, `TOOL-034`).

### History options and the Edit History Log

`History Options` (panel menu) toggles:

- `Automatically Create First Snapshot` — snapshot the initial state when the document opens.
- `Automatically Create New Snapshot When Saving` — snapshot on every save.
- `Allow Non-Linear History` — changes are appended at the end of the list instead of deleting later states, so a state can be changed and deleted individually.
- `Show New Snapshot Dialog By Default` — always prompt for a snapshot name.
- `Make Layer Visibility Changes Undoable` — record layer visibility toggles as history steps (off by default; `LAY-011`).

Separately, `Edit > Preferences > General > History Log` records a textual edit log (`Metadata`, `Text File`, or `Both`; `Sessions Only`, `Concise`, `Detailed`). Storing the log as metadata increases file size and slows open/save; the log can be digitally signed for tamper evidence (`10-workflow-io/file-info-and-metadata.md`).

## UI surface

| Location | Type | Shortcut (Win / Mac) | Notes |
|---|---|---|---|
| `Window > History` | Menu → panel | — | Display the panel; or click the History tab. |
| Panel row left column | Set brush source | click | Snapshot/state source for the History Brush. |
| Panel snapshot slider | Navigate snapshots | drag | Moves up/down between snapshots. |
| State name | Jump | click | Reverts the image to that state. |
| `Step Forward` / `Step Backward` | Menu | `Ctrl+Shift+Z` / `Ctrl+Alt+Z` (`Cmd+Shift+Z`/`Cmd+Option+Z`) | Panel or `Edit` menu. |
| Create New Snapshot button | Button | `Alt`/`Option`-click | Options dialog. |
| Create New Document From Current State button | Button | — | New untitled document. |
| Delete icon | Button | `Alt`/`Option`-click a state | Deletes the selected state; `Alt`/`Option`-click an image state (not the current one) duplicates it (Help keys table). |
| Panel menu | Menu | — | `Step Forward/Backward`, `New Snapshot`, `Delete`, `Clear History`, `New Document`, `History Options`. |
| `Edit > Undo` / `Edit > Redo` | Menu | `Ctrl+Z` / `Ctrl+Shift+Z` (`Cmd+Z`/`Cmd+Shift+Z`) | Non-panel sibling commands. |
| `Edit > Purge > Histories` | Menu | — | Purges all documents; not undoable. |
| `Edit > Preferences > General` | Preference | — | History Log toggle and options. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| History states remembered | int | 20 | 1 … 1,000 (preference) | `Edit > Preferences > Performance > History States`; older states auto-deleted to free memory. |
| Automatically Create First Snapshot | bool | on | on / off | Initial-state snapshot. |
| Automatically Create New Snapshot When Saving | bool | off *(Help: option exists; default not stated)* | on / off | Snapshot on save. |
| Allow Non-Linear History | bool | off | on / off | Changes append instead of truncating. |
| Show New Snapshot Dialog By Default | bool | off *(not stated)* | on / off | Always prompt for name. |
| Make Layer Visibility Changes Undoable | bool | off | on / off | Layer eye toggles become states. |
| Snapshot From | enum | Full Document | Full Document / Merged Layers / Current Layer | Snapshot contents. |
| Snapshot name | string | auto | — | Dialog or inline rename. |
| History Log | enum | on, Metadata *(per Help)* | off / Metadata / Text File / Both | Preference. |
| Edit Log Items | enum | Sessions Only | Sessions Only / Concise / Detailed | Preference. |

Defaults marked "not stated" are unresolved; the Help documents the options but not every default.

## Algorithms & pipeline

The panel is a **view + controller over the `ARCH-009` history engine**; it stores no authoritative history.

1. **State model.** Each committed command appends a `HistoryState { id, label, snapshot_kind, undo_record }`. The engine owns the memory (per-tile backups, structural deltas) and the 20-state ring; the panel renders the list and drives selection.
2. **Non-linear vs linear truncation.** On a change after a selected state: linear mode truncates all later states first; non-linear mode appends the new state at the end and leaves the others. The panel dims later states only in linear mode.
3. **Jump.** Selecting a state reverts the document to it by replaying/inverting the intervening records; it does not delete records until a new edit is committed (the Help notes the eliminating change is undoable).
4. **Snapshots.** A snapshot duplicates the current state's materialisation (full document, merged, or current layer) into a named, long-lived record outside the ring; it is discarded on document close.
5. **New document / drag to document.** Materialise the selected state into a new or existing document; the new document's history contains only `Duplicate State`.
6. **Brush source.** The panel exposes the selected snapshot/state id as the History Brush source to `TOOL-033`/`TOOL-034`.
7. **Purge.** `Clear History` drops the list without freeing; `Alt` `Clear History` and `Edit > Purge > Histories` free the underlying memory and are not undoable.
8. **Edit History Log.** A parallel textual journal is emitted per state when the preference is enabled (`ARCH-009` + `10-workflow-io/file-info-and-metadata.md`).

## Rust module mapping

Design proposal; the engine is `ARCH-009`.

- `pictura_core::history::{History, HistoryState, Snapshot}` — `History { states, snapshots, ring_limit, mode: Linear | NonLinear }`.
- `pictura_core::history::commands` — `JumpToState`, `StepForward`, `StepBackward`, `DeleteState`, `ClearHistory { purge: bool }`, `NewSnapshot { from }`, `RenameSnapshot`, `DeleteSnapshot`, `NewDocumentFromState`, `PurgeAllDocuments`.
- `pictura_core::history::EditLog` — textual log emission gated by preferences (`SessionsOnly | Concise | Detailed`).
- `pictura_ui_bridge::HistoryViewModel` — projected rows `{ state_id, label, snapshot: bool, is_current, brush_source, is_snapshot_source }`.
- `pictura_ui_bridge::HistoryBrushSource` — the state/snapshot id consumed by the brush tool specs.

Crossing types: `StateId`, `SnapshotId`, `SnapshotFrom`, `RingLimit`, `bool` preference flags. No Qt types in `pictura_core`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

| Proposal | Base | Responsibility |
|---|---|---|
| `HistoryPanel` | `QDockWidget` | Host; snapshot list + state list + bottom buttons + panel menu. |
| `HistoryModel` | `QAbstractItemModel` | Snapshot and state rows; roles for label, kind, current, brush source, dimmed. |
| `HistoryRowDelegate` | `QStyledItemDelegate` | Left brush-source column, snapshot thumbnail, dimming, slider column. |
| `SnapshotDialog` | `QDialog` | Name + `From` (Full/Merged/Current Layer). |
| `HistoryOptionsDialog` | `QDialog` | The five option toggles. |
| `NewDocumentFromStateAction` | `QAction` | Button/menu/drag wiring. |
| `ClearHistoryAction` | `QAction` | Distinguishes `Clear` from `Alt`-purge (memory freeing). |
| `HistoryPanelMenu` | `QMenu` | Panel-menu actions; labels reflect `Allow Non-Linear History`. |

## Data-model impact

- **Runtime-only.** States, snapshots and the history list are never serialised into the document (the Help: states/snapshots are cleared on close; snapshots are not saved with the image).
- **`History Log` metadata** is the only persistent artefact and belongs to `10-workflow-io/file-info-and-metadata.md`; it is not the panel's list.
- **Layer-visibility states** depend on the `Make Layer Visibility Changes Undoable` preference and the `LAY-011` visibility model.
- **Undo granularity** is `ARCH-009`'s: one state per committed command; live drags coalesce; non-linear mode changes truncation semantics only.
- **Memory** is governed by the ring limit and the tile-backup strategy (`ARCH-006`, `ARCH-009`); `Alt` `Clear History` and `Edit > Purge` are the explicit release valves.

## Edge cases

- **Document close/reopen:** states and snapshots vanish; the panel must not imply persistence.
- **Ring overflow (> 20):** the oldest states drop automatically; snapshots survive the ring.
- **Non-linear deletion:** deleting a middle state must leave later states that depend on it in a consistent (re-evaluated) form; `ARCH-009` must define the semantics.
- **Snapshot after a state that later changes:** a snapshot is an independent materialisation and must not be invalidated by truncation in linear mode.
- **New document from a merged/current-layer snapshot:** only that composition is carried; other layers are absent.
- **Memory pressure:** `Clear History` does *not* free memory, so a low-memory condition must offer the `Alt`-purge path, with its no-undo caveat.
- **Brush source deleted:** if the source state/snapshot is deleted, the History Brush source must fall back deterministically and be reported.
- **Empty history / first state:** opening a document with `Automatically Create First Snapshot` off yields no snapshot row; the Delete button and `New Document From State` must disable when nothing is selected.
- **32-bit / PSB:** snapshots of large floating-point documents must respect the memory cap and may be compressed or refused with a clear message.
- **GPU unavailable:** pure model operation; unaffected.

## Parity acceptance criteria

1. Given a document, `Window > History` lists states oldest-to-newest with the tool/command labels; program-wide changes never appear.
2. Given the default ring of 20, applying more than 20 changes drops the oldest states and keeps the most recent 20.
3. Given a selected middle state, changing the image eliminates all later states in linear mode; `Undo` restores them. With `Allow Non-Linear History`, the change appends and no state is lost.
4. Given `Automatically Create First Snapshot` on, the initial state appears as a snapshot at the top; turning it off, a fresh document has no snapshot row.
5. Given `New Snapshot` with `From = Merged Layers`, the new document created from that snapshot contains the flattened composite only.
6. Given a state/snapshot dragged onto the `Create a New Document From Current State` button, the new document's history contains only `Duplicate State`.
7. Given the left-column click on a state, the History Brush paints from that state to the current image at the same location (`TOOL-033`); a merged snapshot paints across layers.
8. Given `Alt`-click `Clear History`, the list is purged and memory freed and the action cannot be undone; plain `Clear History` leaves memory unchanged.
9. Given `Make Layer Visibility Changes Undoable` on, toggling a layer eye adds a history state; off, it does not.
10. Given the History Log preference enabled, an edit log is emitted per the selected `Edit Log Items` detail level and is not the panel's state list.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Using the History panel" (states per change; jump to a state; `Window > History`; panel anatomy A–D; program-wide changes excluded; default 20 states and preference; cleared on close; default initial snapshot; oldest-top ordering; dimming; linear truncation and its undo; `Allow Non-Linear History`); "Revert to a previous image state" (click name; `Step Forward`/`Step Backward`); "Delete one or more image states" (Delete/truncate; Clear History vs `Alt` purge; `Edit > Purge > Histories`); "Create or replace a document with an image state" (button/menu/drag; `Duplicate State`-only list; drag onto existing document; saving snapshots across sessions); "Set history options" (all five options); "Set Edit History Log options" (Metadata/Text File/Both; Sessions Only/Concise/Detailed; metadata-size and signing notes); "Make a snapshot of an image" (name, session lifetime, comparison/recovery uses, not saved with image, non-linear interaction; Create New Snapshot; `From` Full Document/Merged Layers/Current Layer; work with snapshots); "Paint with a state or snapshot of an image" (History Brush source via the left column; same-location same-layer copy; merged snapshot); "Use the Undo or Redo commands" and "Revert to the last saved version" (Revert as a state); "Keys for the History panel" (`Alt`+New Snapshot; double-click rename; `Ctrl+Shift+Z`/`Ctrl+Alt+Z`; `Alt`-click state duplicates; `Alt`+Clear History no-undo).

Consulted as search-result snippets only (not individually fetched): none specific to this panel. `helpx.adobe.com` returns 403 and was not used.

Fetched for this revision:

- `https://web.archive.org/web/20180801214950/https://helpx.adobe.com/photoshop/using/performance-preferences.html` — archived Adobe Performance-preferences page (CC-era, used only for the long-standing preference semantics): history is capped at 1,000 states with a default of 20, configured under History & Cache > History States; cache levels default 4.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/History_palette.html` — Martin Evening CS6 support page: the number of recorded history states is configured in the Photoshop Performance preferences; default-panel dimming and `Allow Non-linear History`; the current-history-brush icon and `Edit > Purge > Histories`.
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Essentials workspace: secondary icon column contains **History** (top) and **Properties**; History is also carried into the Painting workspace.

## Open questions

- **Defaults for `Automatically Create New Snapshot When Saving` and `Show New Snapshot Dialog By Default`.** The Help documents the options but not their initial values. *Resolves with:* a CS6 preferences dump/capture.
- **Non-linear deletion consistency.** How `ARCH-009` re-materialises states that depended on a deleted middle state is undefined here. *Resolves with:* an ADR in `01-architecture/undo-history.md`.
- **New-document-from-state layer/mask fidelity** (does it carry masks, effects, smart objects?) is not stated. *Resolves with:* a CS6 test.
- **Snapshot storage cost** (full raster vs references) and whether all three `From` modes are always offered in all color modes is not specified. *Resolves with:* `ARCH-009` design + CS6 test.
- **History Log file path/persistence** belongs to `10-workflow-io/file-info-and-metadata.md`; the panel exposes only the state list.
