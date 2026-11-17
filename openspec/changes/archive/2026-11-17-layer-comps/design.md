# Design

## Context

See `proposal.md` for motivation and the capability deltas for the behavior
contract.

The current engine already provides the pieces this feature needs to sit on:

- `Document` owns document-level metadata (`slices`, `annotations`, paths,
  image resources) that rides history snapshots and cloning. `Layer.extra_blocks`
  preserves every additional-layer block the codec does not model, so `lyid`,
  `shmd`, and `fxrp` already survive read/save.
- `Document.image_resources` is preserved verbatim and `image_resources.rs`
  already decodes, frames, and replaces typed resource blocks.
- Layer mutations live in `pictura-render::document_ops`; `translate_layer_path`,
  `set_layer_style_value`, and `resolve_path` are available there.
- The app commits edits through `PictureView::edit(label, ...)`, which captures
  one history state and drives the recomposite.
- Panels are plain `QWidget` content widgets hosted by `PanelColumn`, reached
  through the `panelToggles` registry and `Window > Panels`.
- `Export Layers to Files` already supplies a dialog, a cancellable batch, a
  scratch-document path, and the PSD/raster/PDF save edge.
- The PDF writer builds a single page today and has a page-building body that can
  be factored into a reusable step.
- `pictura-core` has no codec dependency, so image-resource parsing stays in
  `pictura-codec`; the core identity allocator takes an explicit seed.

The PSD comp format was established from the public `psd.js` parser and
black-box inspection of a public Photoshop-authored sample. The observed schema
is:

```
resource 1065: version-16 descriptor
  class: "CompList"
  "list": VlLs [
    Comp { "Nm  ": TEXT, "compID": long, "capturedInfo": long }
  ]

per layer: additional-info block "shmd"
  entries: sig "8BIM", key, u64 length, data
  entry "cmls", version-16 descriptor, class "null":
    "origFXRefPoint": Objc { "Hrzn": double, "Vrtc": double }
    "LyrI": long                       (matches lyid)
    "layerSettings": VlLs [
      Objc { "enab": bool,                              // visibility
             "blendOptions": Objc { "Md  ": enum },     // appearance
             "FXRefPoint": Objc { "Hrzn", "Vrtc" },     // position
             "vectorMask": Objc { "Ofst": { ... } },
             "layerSpecific": Objc { ... },
             "compList": VlLs [ long compID ] }         // 0 = base state
    ]
```

`capturedInfo` carries the recorded-class bits (1 visibility, 2 position, 4
appearance). The observed sample records visibility only, so position and
appearance key semantics remain partly inferred. The `shmd` entry list's 8-byte
length is specific to that block; it is not the general tagged-block framing.

## Goals / Non-Goals

**Goals:**

- Make comps a durable document feature: create, apply, update, delete, rename,
  options, previous/next, Last Document State, and warnings.
- Keep comps interoperable with PSD/PSB without regressing the existing
  lossless round-trip: untouched comp bytes are re-emitted, comp-free files are
  byte-identical, and ids are assigned lazily.
- Keep the change inside the existing seams: document metadata, render ops,
  codec resource handling, the app bridge, panel host, and export tools.
- One reviewable PR that includes the panel, persistence, and both scripts.

**Non-Goals:**

- WPG export (requires the optional Goodies plug-in).
- Drag-to-create and drag-to-delete comps.
- Recordable Actions/script commands for comp create/apply/update/delete.
- Smart-Object-internal comps and post-CS6 bulk update across comps.
- A new `Layer` identity field or a document-wide node arena.
- Streaming multi-page PDF output; the first version builds pages in memory.
- Full LAY-022 parity where the docs name interactions not shipped here; those
  omissions are recorded in the parity register.

## Decisions

### D1: Comp store lives in `Document` with copy-on-write sharing

The store is document metadata, like `slices` and `annotations`, so history,
recovery, undo, and PSD saving already carry it. The comp list and Last Document
State are held behind `Arc` and forked with `Arc::make_mut` on edit, so a history
snapshot of a document with many comps shares entries instead of cloning them.

`LayerCompStore` separate semantic data (`comps`, `last_state`, `next_comp_id`)
from bookkeeping (`parsed`, `dirty`, `base_dirty`, `ids_dirty`). Its `PartialEq`
compares only semantic data, so two equal documents cannot differ merely because
one was read and one was constructed. Comp ids are allocated as max+1 and are
never reused after a delete.

Alternative: keep the store in `DocumentSession`. Rejected because exports,
recovery, and PSD save all read the document, and metadata edits would not ride
history automatically.

### D2: Stable identity comes from `lyid`; no new `Layer` field

`lyid` is present on every Photoshop layer (including groups) and is already
preserved in `extra_blocks`. A core helper reads it as a big-endian `u32` and
writes it back into `extra_blocks`. A core allocator assigns identities from an
explicit seed; it does not parse image resources.

Identity assignment is lazy: it runs when a comp is created, captured, or
applied, and after a structural edit on a document that already has comps. It
does not run on every open. A comp-free PSD without `lyid` therefore saves
byte-identically. `pictura-codec` reads resource 1044 and the largest `lyid`,
then passes the seed to the allocator; it advances 1044 when identities were
added or changed.

Duplicate and paste rewrite the copy's `shmd`/`cmls` so the copy carries no
source `LyrI` and is not captured by an existing comp. Converting a captured
layer to Background invalidates its captured identity (it clears or rotates the
`lyid`), so the comp warns instead of silently resolving a changed layer.

Reordering and renaming never touch ids. Normalization after structural edits is
a single `apply_edit` choke point rather than a change to every creation path.

Alternative: a first-class `Layer.id` field. Rejected: hundreds of
`Layer { ... }` literals (most of which would need a default) and a codec field
for information the file already carries.

### D3: Capture is a per-layer entry with optional class fields

Each comp holds an ordered list of entries keyed by layer id. An entry carries
only the fields its comp records: `visible: Option<bool>`,
`position: Option<Position>`, `blend: Option<BlendKey>`, and
`style_visible: Option<bool>`. `BlendKey` preserves an unrecognized 4-byte PSD
key as well as a known `BlendMode`. A field that is not recorded is absent, so
apply cannot accidentally write it. Layers added after capture have no entry and
keep their current state. Smart Filter state is never captured.

Appearance records the blend key/mode always when the class is enabled, and the
`lfx2` master switch only when the layer carries an `lfx2` style. A layer with no
style or a legacy `lrFX` style captures no style state; its appearance still
covers the blend mode.

### D4: Position is an anchor delta, maintained on every translation

A layer's position anchor is its `fxrp` block when present, otherwise its rect's
left/top (a project convention for engine-created layers). Reads update the
in-memory anchor from `origFXRefPoint` when the file value differs. Every layer
translation updates the anchor by the same delta: move, transform,
align/distribute, nudge, and comp apply. The update lives in the shared
offset/transform helpers, not only in `translate_layer_path`.

Capture stores the anchor. Apply computes an exact integer delta to the current
anchor, translates the layer through a comp-specific helper, and keeps the
anchor in sync. Repeated apply/undo must be drift-free; stored anchors are exact,
not accumulated rounded fractions. PSD writes the captured anchor as
`FXRefPoint`.

Apply is top-down with ancestor suppression: a captured group's move consumes
its captured descendants, so a descendant entry already moved by an ancestor is
skipped. Capture may store entries for every layer, but each layer moves exactly
once. Groups are expanded to their descendants; adjustment layers translate
their mask; shape layers translate their outline through the existing
shape-aware path.

Alternative: capture the rect only. Rejected because Photoshop stores the
reference point, and a reopened Photoshop file needs the same delta to move the
layer.

### D5: Apply is one full recomposite; metadata edits are `Edit::Meta`

Applying a comp writes recorded fields across many layers; a full recomposite is
correct and simple. It records one state labelled `Apply Layer Comp`. Last
Document State restore is another `Edit::Full`. Create, update, delete, rename,
options, duplicate, and warning clears are `Edit::Meta` with labels such as
`New Layer Comp`, `Update Layer Comp`, `Delete Layer Comp`.

Apply computes its outcome before mutating. A comp whose entries are all missing
returns `Edit::None`: no recomposite, no history state, and no Last Document
State update. `ponytail:` a future optimization may bound the damage with
`damage::influence_paths`; comps commonly touch most of the tree, so full is the
safe default. This O(layers) refresh is recorded as a parity/perf gap against
the docs' O(captured entries) target.

### D6: Last Document State is in-memory, with a refreshed base variant

Before each apply that changes the document, the engine captures the layer state
across all three classes for every current layer and stores it as Last Document
State. It is updated only on a change-making apply, so a no-op apply leaves the
previous state (or its unavailability) untouched. It is not a comp and cannot be
edited or deleted.

On save, a base `cmls` variant tagged `compList: [0]` is written from the current
layer state. Applying a comp marks the base variant dirty so a later clean save
refreshes it; otherwise a save after apply could re-emit a stale base state. On
open, the base variant is not treated as a comp and does not enable Last
Document State, which still requires an apply in the session. This is the
conservative reading of the unverified persistence question.

### D7: Warnings are computed from missing ids and persisted as a project extension

A comp warns when any captured id no longer resolves. Clearing records the
currently missing ids for that comp; clearing all records them for every comp
and records one metadata state. A cleared id is pruned when it resolves again
(for example after undo), so deleting the same layer once more raises the
warning again. Converting a layer to Background invalidates its identity, so it
takes the same warning path.

To make warning state survive save/reopen, each comp object carries two
project-owned descriptor keys: the captured id list and the cleared-id set.
Photoshop ignores unknown descriptor keys; Pictura reads them back. The standard
`Nm  `, `compID`, and `capturedInfo` keys remain the interchange format.

Alternative: warn only in-session. Rejected because the warning is part of the
saved comp's meaning and the issue expects the caution state to persist.

### D8: PSD persistence uses a clean/raw and dirty/rewrite split

The store records whether the comp model was parsed and whether it has been
edited. On open, the codec:

1. Reads resource 1065 into comp records.
2. Reads each layer's `shmd`/`cmls` variants and attaches them to the comps by
   `compID`, ignoring `compList: [0]`.
3. Seeds the id counter from resource 1044 and the largest `lyid`.
4. Updates the in-memory `fxrp` anchor from `origFXRefPoint` when they differ.
5. Leaves `Layer.extra_blocks` and `Document.image_resources` untouched.

On save:

- Comp model clean and base not dirty: the resource path may rewrite
  ICC/path/guide blocks, but it must preserve 1065; `extra_blocks` is re-emitted,
  and the 1065/`shmd` payloads round-trip exactly, including unknown keys. The
  comp-free path assigns no ids, so a file without `lyid` is unchanged.
- Comp model clean but `base_dirty` (after an apply): rewrite only the `compID 0`
  base variant from the current layer state; leave 1065 and the comp variants
  unchanged.
- Comp model dirty: the resource chain replaces/removes 1065, and the layer
  writer replaces each participating layer's `shmd` with a rebuilt `cmls` while
  preserving the block's other entries (`cust` and unknown entries). Unknown
  keys inside a rewritten `layerSettings` variant are merged from the original
  variant with the same `compList`. A layer no comp captured keeps its block,
  except when the store is dirty and empty: then every layer's `cmls` is removed
  while its other `shmd` entries remain.
- Comp parse failed: the store stays clean/unparsed and the original bytes are
  preserved.

Dirty cmls variants carry `enab`, `blendOptions.Md`, `FXRefPoint`,
`vectorMask.Ofst`, `layerSpecific`, and `compList`, plus project keys for the
style switch, warning data, and an unknown raw blend key when needed. The
`shmd` entry list's 8-byte length is specific to that block.

### D9: The panel is a list widget with a custom row delegate

`LayerCompsPanel` uses a `QListWidget` in `ExtendedSelection` mode and a
`QStyledItemDelegate` that paints three zones per row: the apply glyph, the
name, and the caution glyph. An event filter on the viewport dispatches clicks
in the apply zone to apply, clicks on the caution zone to the warning menu, and
sends the rest to normal selection. Double-clicking the name begins inline edit
through the delegate's editor; double-clicking the rest of the row opens the
options dialog. This reuses the repo's proven panel patterns (History's leading
icon zone, Paths' list/menu, Layers' inline editing) without introducing a model
class for a short list.

Last Document State is selectable but read-only: update, delete, rename, and
options are disabled or refused. The panel clears its rows when the active
document closes or is replaced and writes nothing to the old document.

The panel gets its own `LayerCompOptionsDialog` (name, comment, three checks)
and a panel menu hook. It is registered in the panel host's overflow group,
hidden by default, and reached through a checkable `Window > Panels > Layer
Comps` toggle. The `panelToggles` table gains the toggle; `menu_registrar` calls
`setImplemented` at runtime. User-visible labels use `tr()` so the default comp
names and the Last Document State label follow the application's localization
path. Qt tests assert the history depth for apply, Last Document State, Clear
All, and the metadata edits (Options, Duplicate, Delete key).

### D10: The bridge exposes small per-row calls and one action per operation

A new `cxxqt_object/layer_comps.rs` bridge module (declarations only) plus
`imp.rs` exposes free functions over `PictureView`: row count, id, name,
comment, record bits, warning, last-state availability; create, apply, update,
delete, rename, set options, duplicate, clear warning(s), restore last, and the
export entry points. Mutations use `Pin<&mut PictureView>` and `edit()`. The
panel keeps selection and dialog defaults; the bridge keeps document truth.
C++ gets a thin `api/layer_comps_api.{h,cpp}` membrane over the generated
bridge header.

The bridge file MUST be added to `crates/pictura-app/build.rs` `.files([...])`;
`build_config` checks that list against the bridge files on disk and rejects
one/two-function bridges. `cxxqt_object.rs` is at its 1204-line ceiling, so a
line must be freed or a declaration split before adding `mod layer_comps;`.

### D11: Exports clone and apply, never touch the live view

`export_layer_comp_file` clones the document, applies the comp to the clone,
recomputes the clone's composite, and writes it through the existing export save
edge. A shared helper is extracted from `export_layer_file` so PSD, raster, and
PDF output stay identical. The Files batch reuses `ExportLayersDialog` with a
comps mode that hides Visible Layers Only, and `runCancellableJob` for progress
and cancel. The export refuses or settles an active transform/edit session before
reading the document. `scratch_document` is set to a dirty-empty comp store and
strips 1065/cmls so exported single-layer PSDs do not carry stale comps.

Names sanitize the prefix and comp name separately, preserve the prefix during
truncation, and replace an existing file at the computed path (matching Export
Layers). The comp-name part falls back to `Layer Comp`.

For PDF, `pdf::write_pdf` is split into a page-building function and a
multi-page entry point `write_pdf_pages`. The multi-page `Built` result holds a
`Vec` of page roles and writes `/Pages` with every page in `Kids` and the real
`Count`; page objects are numbered in page order and each page's resources are
local. Single-page output keeps the current path (including linearization);
multi-page output uses the plain writer and builds all pages in memory. The
comps PDF uses default options with `preserve_editing` and `layers` off and ICC
on. Both commands call `setImplemented` and have enabled providers so their menu
items are live.

Alternative: apply comps to the live document and restore. Rejected: a crash or
cancel mid-export would leave the document mutated and history polluted.

### D12: Commands and icons follow the catalogue

`window.panels.layer.comps` and `file.scripts.layer.comps.to.files` already
exist as disabled catalogue rows. `file.scripts.layer.comps.to.pdf` is added to
`entries.rs`; the goldens (`catalogue.tsv`, `const_names.tsv`) and
`command_ids.h` are regenerated. The panel row becomes `checkable: true` so the
generic toggle's `action->isChecked()` can show the panel; the recording flag
stays false and `setImplemented` runs at runtime. The export commands call
`setImplemented` and gain enabled providers.

Handlers live in `frame_commands.cpp`; the panel toggle handler is the generic
one in `menu_registrar.cpp`. Icons named exactly by command id are added for the
three commands, plus a `layercomps.warning` icon; the panel buttons reuse
existing icons. The new command ids are added to `kIconCommands`, and
`docs/dev/icon-provenance.md` is regenerated from the icon map. WPG has no row
because the plug-in is absent; this and the omitted drag/actions interactions
are recorded in the parity register.

### D13: Tests use generated fixtures; the real Photoshop sample is manual

`CONTRIBUTING.md` forbids committing Photoshop-authored assets. Codec tests
write their own comp PSDs and hand-build Photoshop-style descriptors to exercise
the exact keys; the public sample stays a local manual oracle. Engine tests drive
capture/apply on synthetic layer trees. Qt Test drives the panel and exports
with `ScopedStateHome` and the existing fixture helpers. Every spec scenario is
mapped in `test-plan.md`; the manual rows cover a Photoshop-authored comps file
and Photoshop's tolerance of the project-owned warning/comment keys.

## Risks / Trade-offs

- **Position/appearance PSD keys are inferred** from one visibility-only sample.
  The engine's own round-trip is fully tested; Photoshop-visible position and
  style behavior needs the manual round-trip row. If it diverges, the codec
  write keys and design are revised together; do not claim parity before it.
- **The comment key is not observed.** Pictura writes a project-owned
  `KkCm` text key and reads common candidates (`comment`, `Cmnt`, `Cm  `).
  Photoshop comment parity is not claimed until verified.
- **The multi-page PDF refactor touches a heavily tested writer.** Mitigation:
  extract the page builder without changing single-page output and keep the
  existing PDF tests as the regression net.
- **In-memory multi-page PDFs scale with comp count × document size.**
  `ponytail:` noted; a streaming writer is the upgrade if large comp sets matter.
- **Variant-level unknown-key merging adds codec complexity.** The raw-source
  path still covers untouched files; the merge covers dirty rewrites.
- **Project-owned warning/comment keys are not Photoshop-standard.** They are
  ignored by Photoshop and preserved by Pictura. The standard keys still carry
  names, ids, recorded classes, and captured state.
- **Full recomposite on apply is expensive on very large documents.**
  Correctness first; `damage::influence_paths` is the bounded-refresh upgrade.
  The divergence from the docs' O(captured entries) target is a parity gap.
- **Id normalization is lazy**, so comp-free files are unchanged. A comp-bearing
  file that lacked `lyid` changes on the first save, which is compatible.
- **Resource 1072 group state is not read or written.** The docs name it as a
  possible group-state vehicle; it stays preserved opaque, and a group-comp
  sample is the verification upgrade.
- **Actions recording, drag duplication/deletion, and WPG are omitted.** They
  are recorded as parity gaps rather than silently claimed.

## Migration Plan

No data migration. Old PSDs open with an empty comp list; PSDs with comps are
read, displayed, and (until edited) re-saved with their original comp bytes.
Comp-free files without `lyid` are byte-identical because ids are lazy. Session
stores need no change because comps live in the document. Archive the change in
the same PR so `openspec/changes` stays zero-open.

## Open Questions

- Exact Photoshop behavior for position and style capture keys (manual
  round-trip will settle it; the codec keys and design are provisional until
  then).
- The comp comment key Photoshop itself uses (project-owned fallback in place).
- Whether Photoshop treats the `compList: [0]` base variant as Last Document
  State on reopen (we keep Last Document State in memory and refresh the base
  variant on apply).
- Whether resource 1072 carries group-comp state in addition to `cmls.enab`.
- Actions recording, drag duplication/deletion, and WPG are tracked parity gaps,
  not open design questions.
