# Proposal

## Why

The layer-stack invariants — an unclipped container bottom, clips that do not
base on the Background or dangle without a base, and a bottom-most and unique
Background — are re-applied by hand at each structural mutator. Two recent
stack regressions (#658, #670) were a mutator that forgot one. Several writers
already bypass reconciliation (`merge.rs` splices and the Stamp rollback,
smart-object conversion, Layer via Copy/Cut, selection lift, and direct
`doc.layers.push` in the app and paint crates), so whether the stack is valid
depends on which command ran.

The last-layer rule is a separate command invariant already owned by
`ui/layers-delete-guard`; this change does not take it over.

## What Changes

- Introduce a `LayerStack` type in `pictura-core` that owns both
  `Document.layers` and every `Layer.children`. Reads stay slice-like; all
  structural edits (insert, remove, move, splice, replace, group, ungroup,
  reverse, truncate, retain) exist only as stack methods.
- Move clipping and Background reconciliation into that module. Each structural
  edit is atomic: it re-establishes the invariants once, before returning, and
  a refused edit leaves the tree unchanged.
- Add a side-effect-free well-formedness check and debug-assert it after every
  structural edit, but only when the pre-edit tree was itself well-formed.
- Keep a raw install path (`from_layers`) for the codec, scratch documents, and
  test fixtures. History snapshots rely on `Clone`. A malformed PSD still loads
  as authored; the first structural edit normalizes only the clipping in the
  containers it touches.
- Migrate every structural writer in `pictura-render`, `pictura-app`,
  `pictura-paint`, and `pictura-codec` to the stack API.
- Fix the un-normalized sites so merge, smart-object conversion, Layer via
  Copy/Cut, and selection lift no longer leave a baseless clip. An unclipped
  insertion between a base and its clipped layers may become the run's new
  base; that is a legal positional re-base, not a base removal.
- **BREAKING (internal API)**: `Document.layers` and `Layer.children` change
  from `Vec<Layer>` to `LayerStack`; direct `push`/`insert`/`remove` calls must
  become stack methods.
- The residual bypass is explicit: `from_layers`, whole-node assignment through
  `IndexMut`, and the still-public `Layer.clipping`/`Layer.background` fields
  can construct an invalid tree outside a structural edit. The check and the
  debug assertion cover structural edits, not arbitrary flag writes.

## Capabilities

### New Capabilities

- `document/layer-stack-invariants`: the post-edit well-formedness contract for
  every structural edit, the permissive load rule for malformed stacks, and the
  checkable validation surface.

### Modified Capabilities

None. The new capability owns the cross-cutting postconditions, including the
merge and smart-object cases that the command-specific specs did not enumerate.
The existing requirements in `compositing/clipping-masks`,
`compositing/layer-management`, and `ui/layers-delete-guard` keep their current
text.

## Impact

- `crates/pictura-core`: new `layer_stack` module; `Document.layers` and
  `Layer.children` field types; `from_rgba` construction.
- `crates/pictura-render/src/document_ops`: all structural layer ops, merge,
  smart object, via, move-content, document-scope scratch builders, and the
  bitmap-mode clear.
- `crates/pictura-app`: bridge pushes and pops, history snapshots, the
  duplicated `truncate_above`/`strip_adjustments` scratch helpers, and test
  fixtures.
- `crates/pictura-paint`: production pushes in fill, eraser, and art history,
  plus scratch documents and test fixtures.
- `crates/pictura-codec`: layer-tree construction, PDF export clearing, and a
  new codec test module for malformed-stack round trips.
- No PSD serialization change, no new dependency, and no change to the edit
  transaction proposed in #739.
