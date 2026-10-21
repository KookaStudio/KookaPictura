# Undo History

- **Spec ID:** `ARCH-009` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the History panel, snapshots, non-linear history, and the History Brush predate CS6; CS6 carries them forward unchanged.
- **Depends on:** `ARCH-008` document-model, `ARCH-006` gpu-rendering-pipeline, `02-ui-ux/panels/history-panel.md`, `03-tools/history-brush.md`, `03-tools/art-history-brush.md`, `10-workflow-io/scratch-disks-and-memory.md`, `11-cross-cutting/crash-recovery-and-autosave.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. CS6-specific numbers not confirmed by a fetched CS6 source
> are marked *(inferred)*.

## CS6 behavior

Photoshop keeps a per-document, in-memory **History** of editing steps. Each
step is a **history state** shown in the History panel (`Window > History`),
named after the operation (for example "Open", "Layer Via Copy", "Gaussian
Blur"). Clicking a state restores the document to that point. In the default
linear model, making a new edit after stepping back discards all states after
the selected one; Photoshop's history is therefore a linear list with a current
cursor, not a general undo tree.

- **Undo/Redo** — `Ctrl/Cmd+Z` toggles the last state; `Shift+Ctrl/Cmd+Z`
  redoes. `Ctrl+Alt+Z` / `Ctrl+Shift+Alt+Z` (or the History panel) step
  backward/forward through multiple states.
- **Snapshots** — the camera icon saves a named copy of the current state. A
  snapshot is not pruned when the state limit is reached, so it survives long
  editing sessions. Snapshots are *not* saved with the document.
- **Allow Non-Linear History** — a History Options toggle. When on, selecting an
  older state and editing appends a new state instead of deleting the states
  after the selected one. This preserves more work but makes the panel
  non-linear (a branch is not drawn; states are appended in recording order).
- **Delete current state** / **Clear History** — Clear History deletes
  non-snapshot states and cannot itself be undone. **New Document from State**
  clones the document at a state.
- **History Brush** (`Y`) — paints from a chosen source state or snapshot into
  the current state. **Art History Brush** does the same with stylized strokes.
  Both create their own history states.
- **History Log** — optionally writes a text log of states. It records the
  *names* of steps only, not image data; it can live in the file's metadata or a
  separate text file. Reopening a document does **not** restore the History
  panel, even for PSD files.

History is bounded by the **History States** preference (per the current
Photoshop Help, 1–1000; the CS6 default is reported as 20 by one source and 50
by another — see `## Open questions`). When the limit is reached, the earliest
non-snapshot states are discarded. Raising the limit increases scratch-disk
usage; the panel's state list is stored in RAM while it fits and spills to
scratch.

*(inferred, publicly documented)* Photoshop does not store a full image copy per
state. It stores image data in **tiles**, and a state references the tile
versions that changed. "Diffusion" copies only the changed tiles forward between
adjacent states, so a brush dab on a 1-pixel area costs a few tiles, not a full
canvas copy. Undo/redo swaps tile references. This tile-diffusion model is not
published by Adobe and is a design assumption here, not a sourced fact.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| History panel | Dock | `F9`-adjacent; `Window > History` | List of snapshots above states; camera icon, trash icon, "new document" icon |
| History panel > fly-out | Menu | n/a | History Options, Clear History, New Snapshot, Delete Current State, New Document, Step Backward/Forward, Snapshot/State view modes |
| History panel options | Dialog | n/a | History Options: Automatically Create First Snapshot, Automatically Create New Snapshot When Saving, Show New Snapshot Dialog, Allow Non-Linear History |
| Edit > Undo / Redo | Menu | `Ctrl/Cmd+Z`, `Shift+Ctrl/Cmd+Z` | Labels the action |
| Edit > Step Backward / Step Forward | Menu | `Ctrl+Alt+Z`, `Ctrl+Shift+Alt+Z` | Multi-step |
| Preferences > Performance | Pane | `Ctrl/Cmd+K` | History States slider 1–1000 |
| Preferences > History Log | Pane | n/a | History Log checkbox; Save Log Items (Metadata / Text File / Both); Edit Log Items (Sessions / Concise / Detailed) |
| History Brush tool options | Options bar | `Y` | Source picker (state/snapshot), mode, opacity, brush preset |
| Art History Brush tool options | Options bar | `Y` (group) | Style, area, tolerance, brush dynamics |
| Edit > Purge | Submenu | n/a | Purge History (frees RAM; not undoable) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| History States | int | 20 or 50 (CS6, unconfirmed) | 1–1000 | Performance preference |
| Allow Non-Linear History | bool | Off | on / off | History Options |
| Automatically Create First Snapshot | bool | On | on / off | History Options |
| Automatically Create New Snapshot When Saving | bool | On | on / off | History Options |
| Show New Snapshot Dialog | bool | On | on / off | History Options |
| History Log | bool | Off | on / off | Preference |
| Save Log Items | enum | Metadata | Metadata / Text File / Both | Preference |
| Edit Log Items | enum | Sessions | Sessions / Concise / Detailed | Preference |
| Snapshot name | string | "Snapshot N" | — | Snapshot record |
| Current state index | int | last | 0..states.len() | Cursor into the linear list |
| History Brush source | enum (state/snapshot) | current | any retained state/snapshot | Tool options |
| History Brush mode/opacity | enum / percent | Normal / 100 | CS6 blend list / 0–100 | Tool options |
| Art History Brush style | enum | Tight Short | Tight/Short, Tight/Medium, ..., Dab | Tool options |
| Art History Brush area | int px | 50 | 0–500 (inferred) | Tool options |

## Algorithms & pipeline

### Linear history versus snapshots

Model the History panel as:

```text
History {
  states: VecDeque<StateId>,   // chronological; first is "Open"
  current: usize,              // index of the applied state
  snapshots: BTreeMap<SnapshotId, StateId>,  // never pruned
  limit: usize,                // History States preference
  non_linear: bool,
}
```

Editing rule:

- `non_linear == false`: any new edit after `current < states.len()-1` truncates
  `states[current+1..]`, then appends the new state.
- `non_linear == true`: the new state is appended; existing states are kept.
- When `states.len() > limit`, drop the earliest states that are not referenced
  by a snapshot, never dropping the base "Open" state while it is the anchor
  (exact retention policy unverified).

Snapshots keep a reference to a state, so pruning must preserve any state still
referenced. A snapshot itself is never discarded by the limit.

### State representation

A state is a *delta*, not a full copy:

- scalar/structural changes to `Node`s (visibility, opacity, blend, parameters,
  order, masks) as `StateDiff` records with before/after values;
- pixel changes as references to the changed tiles in a content-addressed tile
  store;
- a back-reference to the producing `Command` (for the label and for redo).

To restore state `S`, replay or pointer-swap from the nearest cheaper state.
Diffusion operates at tile granularity: moving from `S[i]` to `S[i+1]` copies
only tiles whose version changed. *(inferred, see above.)*

### Memory management

History lives in RAM while it fits and spills to the scratch disk. Budget policy
(proposal): a per-document cap derived from the History States preference and a
global cap from Preferences > Performance, with an LRU eviction of the oldest
non-snapshot tile versions. Because History is not saved, all of it is
reclaimable when the document closes; `Edit > Purge > History` forces it early.
Undo records for destructive pixel operations must retain enough tile data to
restore the previous state exactly.

### Undo granularity

Proposed mapping of user operations to states (to be confirmed against CS6):

| Operation | States created |
|---|---|
| Brush/pencil/eraser stroke | 1 per completed stroke (on pointer release) |
| Clone/heal stroke | 1 per stroke |
| Gradient drag | 1 on release |
| Marquee/lasso selection | 1 per commit (not per drag) |
| Move/transform | 1 on commit |
| Filter / Camera Raw Filter | 1 per apply |
| Adjustment parameter edit | 1 per committed edit (slider release or dialog OK) |
| Type | 1 per commit of an edit session (exact batching unverified) |
| Layer add/delete/reorder/merge | 1 per operation |
| Image mode / bit-depth / size change | 1 per operation (destructive) |
| Smart object content edit | 1 in the parent document when committed; the nested document has its own independent history |
| History Brush / Art History Brush | 1 per stroke |

### Interaction with adjustment layers and smart objects

- Adjustment and fill layers are document nodes; changing their parameters is a
  normal state. Because they are non-destructive, undo restores parameters
  without touching pixels.
- Smart objects contain a nested document. Editing contents opens a separate
  window with its own History; committing back to the parent records one state
  in the parent. Smart-filter edits follow the same rule. The exact commit
  granularity is *(inferred)*.

## Rust module mapping

Proposals:

- `pictura_history::history` — `History` (state list, cursor, snapshots, limit,
  non-linear flag); `push`, `step_back`, `step_forward`, `goto`, `snapshot`,
  `clear`.
- `pictura_history::state` — `State { id, label, command: CommandId,
  diffs: Vec<StateDiff>, tiles: TileDelta }`.
- `pictura_history::diff` — `StateDiff` enum over node scalar/structural fields
  (visibility, opacity, blend, order, mask params, adjustment params), with
  serializable before/after.
- `pictura_history::tile_store` — content-addressed tile versions and a
  `TileDelta` (changed tile ids + new versions); `diffuse(from, to)` applies a
  delta. Backing store is RAM with optional scratch-file spill.
- `pictura_history::command` — `Command` trait (`label`, `apply`, `revert`,
  `mergeable`), the only sanctioned writer to the document; used by every tool
  and filter.
- `pictura_history::snapshot` — `SnapshotId`, name, referenced `StateId`;
  retention pinning.
- `pictura_history::brush` — `HistoryBrush { source: StateRef, mode, opacity,
  mask }`; samples the source state's tiles and composites into the current
  layer, producing a new state.
- `pictura_history::budget` — memory accounting and eviction policy.

Crossing types: `StateId(u64)`, `SnapshotId(u64)`, `TileId(u64)`, and borrowed
tile slices. Commands are the document mutation API; the UI never mutates nodes
directly.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HistoryModel` | `QAbstractListModel` | Snapshots section + states section; roles: id, label, thumbnail, is_snapshot, is_current |
| `HistoryPanel` | `QWidget`/dock content | List view, camera/trash/new-document buttons, context menu |
| `HistoryOptionsDialog` | `QDialog` | The four History Options toggles |
| `HistoryLogPreferences` | `QWidget` | Log toggle, save destination, detail level |
| `HistoryBrushOptions` | `QWidget` (options bar) | Source combo, mode, opacity, brush preset |
| `PerformancePreferences` | `QWidget` | History States spin/slider with memory estimate |

Per `ARCH-003`, the model is GUI-thread-only; history commands execute on a
worker and publish `dataChanged()` only after commit. Thumbnails are requested
asynchronously from the tile store.

## Data-model impact

- History is **not serialized** into PSD/PSB. Reopening a document yields an
  empty history with a single "Open" state; snapshots are also lost.
- The optional **History Log** is the only persistent trace: text in the file's
  metadata (proposal: XMP `photoshop:History`, unverified) or an external text
  file. It stores step descriptions, not image data.
- Undo record shape (proposal):
  `UndoRecord { command: CommandId, label: String, diffs: Vec<StateDiff>,
  tiles: Vec<TileId> }`, plus an inverse for redo. Compound operations may
  register a single record that aggregates child records.
- The document exposes a revision counter per node/tile so dirty-tracking and
  cache invalidation (`ARCH-006`) can compare against a history state.
- Persisted (non-document) state: the History States preference and History Log
  preferences belong to `ARCH-002`/`11-cross-cutting/preference-storage.md`.

## Edge cases

- **Limit reached** — earliest non-snapshot states are dropped; the panel must
  not show a stale cursor. Snapshots pinned by users are never dropped.
- **Non-linear mode** — branching is not visualised; a new state can share a
  parent with an older state. Ensure `current` remains coherent and undo/redo
  follows recording order, not document tree order.
- **Snapshots** — survive pruning but not document close; reselect a snapshot
  then edit and the states after the snapshot's anchor may be truncated unless
  non-linear is on.
- **Clear History / Purge** — irreversible by design; must not be routed through
  the undo system.
- **Memory pressure** — spill to scratch; if the scratch disk is full, fail the
  operation without corrupting history (surface an error, do not silently drop
  states beyond the limit).
- **Huge/PSB documents** — tile diffs keep per-state cost proportional to the
  edit, not the canvas; a full-canvas filter legitimately touches all tiles.
- **32-bit float documents** — tile deltas store `f32`; memory per tile is 4× the
  8-bit case; eviction must account for this.
- **Smart object nesting** — parent and nested documents have independent
  histories; closing a nested document without committing discards its history.
- **Undo during an in-progress stroke** — strokes must be atomic on release;
  `Ctrl+Z` mid-stroke is undefined and should be ignored or cancel the stroke.
- **GPU tile cache** — undo/redo must invalidate GPU tile references
  consistently with the CPU tile store (`ARCH-006`).
- **Crash recovery** — because history is in-memory, crash recovery must rely on
  the autosave path (`11-cross-cutting/crash-recovery-and-autosave.md`), not on
  history.

## Parity acceptance criteria

- Given a fresh document, the History panel contains exactly one "Open" state.
- Given a linear history and the user steps back two states then paints, all
  states after the selected one are discarded and exactly one new state is
  appended.
- Given non-linear history on and the same sequence, no states are discarded and
  one state is appended.
- Given `History States = N` and more than `N` edits, the number of retained
  non-snapshot states never exceeds `N` and the oldest is dropped first.
- Given a snapshot exists, its state is retrievable after more than `N` further
  edits; deleting the snapshot allows its state to be pruned.
- Given a brush stroke touching K tiles, the new history state's stored delta
  references only the touched tiles (not the full canvas), verified by store
  instrumentation.
- Given a destructive full-canvas operation, undo restores every pixel
  bit-exactly at the document bit depth (8/16/32).
- Given `Ctrl+Z` / `Shift+Ctrl+Z`, the History panel cursor and the document
  state stay synchronized.
- Given a document is saved to PSD and reopened, the History panel is empty and
  the file contains no history image data; if History Log is on, the log is
  present in metadata or the text file.
- Given Smart Object contents are edited and committed, exactly one state is
  added to the parent document.
- Given `Edit > Purge > History`, memory for non-snapshot states is released and
  the action is not undoable.

## Sources

Fetched for this document:

- `https://www.bwillcreative.com/how-to-use-the-history-panel-in-photoshop` —
  History panel accessible via `Window > History`; states vs snapshots; when the
  state limit is reached the oldest states are deleted; snapshots persist;
  History States preference under `Preferences > Performance`, range 1–1000;
  increasing it increases scratch-disk use; `Ctrl/Cmd+Z` undo and
  `Shift+Ctrl/Cmd+Z` redo; report that the default was 50 for CC and 20 before.
  Secondary/community source, written for later Photoshop versions.
- `https://glensmith.co.uk/photoshop/history` — each action is recorded as a
  "State"; jump to any state; Snapshots via the camera icon; History and
  snapshots are lost on close, including for PSD; History Log records steps in
  text and can go to metadata or a separate file; `Window > History`;
  `Edit > Undo`. Secondary/community source, written for later Photoshop
  versions.
- `https://manualzz.com/doc/o/v3ux7/photoshop-cs6-manual-undo-and-history` —
  attempt to fetch the CS6 manual's Undo and History section returned HTTP 403.
  The definition of "Allow Non-Linear History" quoted in `## CS6 behavior` was
  read only from a search-result snippet of this page, not from the fetched
  document; treat as unverified.

Not parsed in this pass: `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
(official CS6 Help; fetch exceeded the 5 MB limit) and
`https://www.oreilly.com/library/view/adobe-r-photoshop-r-cs6/9780132966504/ch06.html`
(HTTP 403).

## Open questions

- **CS6 default History States and exact CS6 option labels.** Sources disagree
  (20 vs 50) and describe later versions. Resolve from the CS6 Help PDF
  (History panel / Performance preferences) or a CS6 screenshot.
- **Tile-diffusion storage model.** The tile/version/diffusion description is
  *(inferred)* from community analysis and Adobe patents, not a
  fetched authoritative source. Confirm against a technical write-up before
  committing to the design; otherwise treat as an implementation choice.
- **Retention policy at the limit.** Does Photoshop always keep the base "Open"
  state and all snapshots, and does `current` ever point at a pruned state?
  Resolve with a CS6 experiment.
- **Non-linear editing semantics.** Exact interaction of non-linear history with
  snapshots, redo, and the History Brush is unverified. Resolve from the CS6
  Help PDF.
- **History Log persistence.** Whether the log is written to XMP
  (`photoshop:History`), to the file's `8BIM` resources, or elsewhere, and its
  exact format, is not sourced. Resolve by inspecting a CS6-saved document with
  History Log enabled.
- **Undo granularity of type and dialogs.** How many states a text edit or a
  slider session produces needs a CS6 reference. Resolve with a scripted test.
- **Smart object commit granularity.** Whether editing and saving a smart object
  yields one or more parent states is unverified. Resolve with a CS6 test.
- **History Brush source semantics.** Whether painting from an older state uses
  that state's composite or the layer's pixels is unverified. Resolve in
  `03-tools/history-brush.md`.
