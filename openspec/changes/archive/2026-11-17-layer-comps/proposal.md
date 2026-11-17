# Proposal

## Why

Photoshop CS6's Layer Comps let a designer save, compare, and restore versions
of a layout inside one PSD. Kooka Pictura has no comp model and no panel; PSD
files that contain comps open with the comps inert, and there is no way to
create or apply one. Issue #265 (LAY-022) tracks this gap and the contract in
`docs/05-layers/layer-comps.md`.

The comp-list layout and per-layer container are known from public sources and
black-box inspection: resource 1065 holds the comp list, and each layer's
`shmd` block holds `cmls` variants keyed by the layer's existing `lyid`. Position
and style key semantics remain partly inferred, but the persistence path is
clear enough to make a durable feature feasible now.

## What Changes

- **A document-level comp store** records, per comp: name, comment, recorded
  classes (visibility, position, appearance), and per-layer captured state.
  The store rides the document, history snapshots, and cloning.
- **Capture, apply, update, delete, rename, and options** work in the engine.
  Apply restores only the recorded classes in one undo step; unrecorded fields
  are untouched, and Smart Filter settings never vary by comp.
- **Last Document State** is captured before each apply and can be restored as
  one undo step.
- **Cannot-be-fully-restored warnings** appear when a captured layer is missing
  (deleted, merged, or converted to Background); warnings can be cleared
  individually or all at once and persist with the document.
- **Stable layer identity** uses the existing `lyid` additional-layer block.
  Missing ids are generated and, with image resource 1044, written back; no new
  `Layer` field is introduced.
- **PSD persistence** reads and writes resource 1065 and the per-layer
  `shmd`/`cmls` descriptors. Untouched files re-emit their original comp bytes;
  edited documents write the standard keys and preserve unknown keys where the
  descriptor tree allows.
- **A Layer Comps panel** is added as its own dock: comp rows with apply and
  caution affordances, Previous/Next, Create, Update, Delete, inline rename, the
  Options dialog, multi-selection cycling, and the panel menu. It is hidden by
  default and reached from `Window > Panels > Layer Comps`.
- **`File > Scripts > Layer Comps to Files`** exports one file per comp in PSD,
  JPEG, TIFF, BMP, or PDF form; **Layer Comps to PDF** writes one multi-page PDF
  with a page per comp. Both work on clones and leave the document untouched.
- **WPG export is not offered**: it requires the optional Goodies plug-in and
  the issue asks only for Files and PDF. Recordable Actions commands and
  drag-to-duplicate/delete are also omitted; those omissions are recorded as
  parity gaps rather than claimed as shipped.

## Capabilities

### New Capabilities

- `document/layer-comps`: the document-level comp store, capture/apply/update
  semantics, Last Document State, recorded-class rules, stable layer identity,
  and warning state.
- `codec/psd-layer-comps`: PSD/PSB persistence of resource 1065 and the per-layer
  `shmd`/`cmls` descriptors, plus `lyid` and resource 1044 identity round-trips.
- `ui/layer-comps-panel`: the Layer Comps dock, dialog, commands, menu actions,
  keyboard/mouse affordances, and default layout placement.
- `interop/layer-comps-export`: the Files and PDF scripts, their dialogs,
  naming/format rules, progress/cancel behavior, and document non-mutation.

### Modified Capabilities

None. Existing layer, history, PDF, and export capabilities keep their current
requirements; the new behavior is additive and is specified by the four new
capabilities above.

## Impact

- `pictura-core`: new `layer_comp` types and a `Document` field; `lyid` helpers.
- `pictura-render`: comp capture/apply/update/validation in `document_ops`,
  group/adjustment/shape position translation, `fxrp` maintenance on moves.
- `pictura-codec`: resource 1065 and `shmd`/`cmls` read/write, resource 1044 and
  `lyid` seeding, read/save integration; a multi-page PDF writer entry point.
- `pictura-app` (Rust): a new `layer_comps` bridge module and export targets.
- `pictura-app` (C++/Qt): the panel, delegate, options dialog, export dialog and
  orchestration, API membrane, panel host registration, command catalogue and
  menu handlers, and the new command/warning icons.
- Command catalogue: a new `file.scripts.layer.comps.to.pdf` row plus
  `command_ids.h` and golden-table regeneration.
- Tests: engine and codec unit tests, Qt Test suites for the panel and exports,
  and manual Photoshop round-trip checks for the inferred position/appearance
  keys.
- Docs/roadmap: `ROADMAP.md`, the LAY-022 open questions, and the parity-gap
  register (separate `TASK-ALLOWS-DOCS` commit).
