# History Brush

- **Spec ID:** `TOOL-033`
- **Status:** `Draft`
- **Parity tier:** `Core` — available in CS6 Standard. (Restoring video/animation frames via the History Brush is `Extended-only`.)
- **New in CS6:** `No` — the History Brush, the History panel, and snapshots predate CS6 and are unchanged.
- **Depends on:** `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-006` gpu-rendering-pipeline, `02-ui-ux/panels/history-panel.md`, `03-tools/art-history-brush.md`, `07-color-painting/brush-engine.md`

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Historical storage (tile diffusion) is an inferred design already
> stated in `ARCH-009`; Adobe does not publish it. Behavioral facts are from the
> fetched CS6 Help PDF.

## CS6 behavior

The History Brush (`Y`) paints a copy of a chosen **history state or snapshot**
into the current image window. It "makes a copy, or sample, of the image and then
paints with it." Typical use: snapshot or record a state (e.g. after a filter),
undo it, then selectively paint the effect back into parts of the image.

- The **source** is selected in the History panel: click the left column of the
  state or snapshot. A **brush icon** then appears next to the chosen source.
- The tool copies **from one state or snapshot to another, but only at the same
  location** — it does not offset the source. The Art History Brush is the
  stylized sibling (`03-tools/art-history-brush.md`).
- Unless the source is a **merged snapshot**, the History Brush paints from a
  layer in the selected state to the **same layer** in another state. A merged
  (Full Document) snapshot paints from the flattened state.
- Options bar: set **opacity** and **blending mode**, and choose a **brush** with
  its options. (The CS6 Help's steps set the source in the History panel; a source
  picker also appears in the options bar of some Photoshop versions — see Open
  questions.) The bar also carries **Airbrush**; while it is on, paint builds while
  the button is held still (`07-color-painting/airbrush-and-flow.md`).
- Each completed stroke creates its own history state (per `ARCH-009`).
- The Eraser's **Erase to History** option and `Edit > Fill > History` restore a
  selected area from the source state by the same mechanism.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, History group | Tool | `Y` | History Brush and Art History Brush share `Y` |
| Options bar | Bar | — | Opacity, blending mode, Airbrush, brush preset and brush options |
| History panel | Dock | `Window > History` | Left column selects the **source** state/snapshot; brush icon marks it |
| History panel fly-out | Menu | — | History Options, snapshots, new document from state |
| `Edit > Fill > History` | Dialog | `Shift+F5` | Region restore alternative |
| Eraser tool options | Bar | `E` | `Erase to History` mode |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Source state/snapshot | ref | current/no source | any retained state or snapshot | Chosen from the History panel |
| Opacity | int % | 100 | 0–100 | Per-stroke |
| Blending mode | enum | Normal | CS6 blend list | Standard paint blending |
| Brush preset | preset ref | last used | any tip | Size, hardness, spacing, dynamics |
| Brush options | varies | preset | — | Shape/dynamics from the Brush panel |

## Algorithms & pipeline

The History Brush is a paint tool whose color source is the **source state's tile
data at the same coordinates**, rather than a layer composite or pattern.

1. Resolve the source state/snapshot to a readable tile view.
   - **Full Document / Merged snapshot:** sample the composite of that state.
   - **Otherwise:** sample the corresponding layer in that state (same layer).
2. For each dab pixel `(x, y)`, read the source value `s(x, y)`; the destination is
   the same coordinate `(x, y)`.
3. Composite `s` into the current layer with the brush mask, opacity, and blending
   mode; unaffected pixels keep the current state.
4. Publishing the stroke as a command appends a new history state.

Because the mapping is identity (same location), the tool is a masked
state-to-state copy; the brush mask is the only spatial selector.

Storage: states are deltas over a content-addressed tile store (`ARCH-009`), so
the source lookup is a tile read from the retained version, not a full-image
copy. The source reference must be **pinned** (like a snapshot) for as long as it
is the active brush source, so the state limit does not prune it from under the
tool.

## Rust module mapping

Proposals (the history machinery itself is `ARCH-009`):

- `pictura-history::brush::HistoryBrush` — `HistoryBrushConfig { source:
  StateRef (State | Snapshot), opacity, mode, brush }`; the source-resolving dab
  loop. (Already proposed in `ARCH-009`; this spec fixes its behavior.)
- `pictura-history::tile_store` — `TileRead` of the source version at `(x, y)`;
  `pin(state)` / `unpin(state)` to protect the active source.
- `pictura-history::state::SourceResolve` — resolves a `StateRef` to either the
  composite of the state or the corresponding layer's tiles.
- `pictura-core::command::HistoryStroke` — emitted on stroke release.

Crossing types: `StateRef`, `SnapshotId`, `TileId`, borrowed tile slices. The tool
reads through `pictura-history` and writes only via a `Command`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HistoryBrushOptions` | `QWidget` (options bar) | Opacity, blending mode, brush preset; optional source picker |
| `HistoryModel` / `HistoryPanel` | `QAbstractListModel` / dock | Source selection (left column, brush-icon marker); owned by `02-ui-ux/panels/history-panel.md` |
| `BrushPresetButton` | `QToolButton` | Brush preset picker shared with other paint tools |
| `HistoryThumbnailDelegate` | `QStyledItemDelegate` | Renders state/snapshot thumbnails and the active-source icon |

Models are GUI-thread-only; source thumbnails are requested asynchronously from
the tile store (`ARCH-003`). Source selection writes to the tool session, not the
document.

## Data-model impact

- **History is not serialized** into PSD/PSB (`ARCH-009`); reopening a document
  yields an empty history and no brush sources. Snapshots are also lost.
- The active **source state/snapshot** is tool session state; it is not a document
  field. It must be cleared or re-resolved if the referenced state is discarded.
- **Undo:** one state per completed stroke; the stroke command stores pre-edit
  tiles so undo restores bit-exactly.
- **Pinning:** while a state/snapshot is the brush source it must be retained even
  if the History States limit is exceeded; a snapshot source is already pinned.
- **Composite vs. per-layer source** changes which tiles are read, not the
  document schema.
- The Eraser's `Erase to History` and `Edit > Fill > History` reuse the same
  source-resolution path.

## Edge cases

- **Pruned source.** If the user selects a state that is later pruned, or the
  state limit discards it, the brush must either pin it or refuse to paint with a
  clear message — never read freed tiles.
- **Non-linear history** (`Allow Non-Linear History`) — stepping back and editing
  appends states; the source reference remains valid by id, but the panel cursor
  semantics must stay coherent.
- **Merged vs. layer source.** A merged snapshot paints the composite; a
  layer-scoped source paints only that layer. Both must be supported.
- **Current state as source.** Painting with the source equal to the current state
  is a no-op; it should be allowed but produce no visible change.
- **32-bit HDR** — the History Brush is explicitly listed among tools usable on
  32-bpc HDR images; keep the float buffer. (Art History Brush is not.)
- **16-bit** — supported (the History Brush is not in the 16-bpc exclusion list).
- **8-bit / CMYK / Lab / Grayscale** — per-channel same-coordinate reads; no
  implicit conversion.
- **Huge PSB documents** — tile-local reads; the source version must not force a
  whole-canvas load.
- **Undo during a stroke** — strokes are atomic on release; mid-stroke undo is
  ignored or cancels the stroke.
- **GPU tile cache** — source reads and the stroke's GPU upload must invalidate
  consistently (`ARCH-006`).
- **Memory pressure / scratch full** — pinning a source raises its retention cost;
  if scratch is full, fail the operation without corrupting history.

## Parity acceptance criteria

- Given a document with a recorded state, selecting that state as the source and
  painting copies its pixels at the same coordinates into the current state.
- Given a **merged** (Full Document) snapshot source, the painted pixels match the
  flattened state's composite; given a non-merged source, only the corresponding
  layer's pixels are copied.
- Given a hidden brush stroke, the current state's pixels remain unchanged.
- Given opacity < 100% and a non-Normal mode, the result matches the same mask,
  opacity, and blend applied to a same-coordinate copy.
- Given a completed stroke, undo restores every touched pixel bit-exactly and
  exactly one history state is added.
- Given a source state with more than `History States` further edits, the source
  is either retained (pinned) or the tool refuses with a clear error; it never
  paints from freed data.
- Given a 32-bpc HDR document, the History Brush paints in floating point without
  an implicit 8/16-bit conversion.
- Given the Eraser is set to Erase to History or `Edit > Fill > History` is used
  with a region, the region is restored from the same source as the brush.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop CS6 Help reference (downloaded and text-extracted). Established:
  History Brush paints a copy of one image state or snapshot into the current
  window; source chosen by clicking the left column in the History panel; brush
  icon marks the source; same-location copy; merged snapshot vs. same-layer
  behavior; options bar exposes opacity, blending mode, and brush; `Y` shortcut
  shared with Art History Brush; `Edit > Fill > History` and Eraser `Erase to
  History` as alternatives; History Brush listed among 32-bpc HDR tools and not
  among the 16-bpc exclusions; the snapshot `Full Document / Merged Layers /
  Current Layer` contents. Primary source.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Art_history_brush.html`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers* support page.
  Established the contrast between the History Brush (re-creates the source data)
  and the Art History Brush (stylized strokes). Secondary, CS6.

## Open questions

- **Options-bar source picker in CS6.** The CS6 Help PDF directs the user to the
  History panel to choose the source; some Photoshop versions also expose a
  source picker in the options bar. Whether CS6 has that picker is unverified.
  Resolve from a CS6 screenshot/build.
- **Composite vs. layer source semantics.** The Help says the brush paints "from a
  layer in the selected state to the same layer in another state" unless a merged
  snapshot is chosen; whether a Full Document snapshot is always composite, and
  whether non-snapshot states can be composite, needs a CS6 experiment.
  (Cross-referenced in `ARCH-009` Open questions.)
- **Default opacity/mode and brush.** Not documented for CS6; assume the paint
  defaults (Normal, 100%).
- **Interaction with `Allow Non-Linear History`.** Whether selecting an old source
  and painting truncates or appends states is inherited from `ARCH-009`; confirm
  the brush creates exactly one state.
- **Source pinning rule.** Whether CS6 pins a non-snapshot source against the
  History States limit is unverified; our design pins it. Resolve with a limit-
  exhaustion test.
