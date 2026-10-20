# Design

## Context

See `proposal.md` for motivation. The current invariant code is split across
`crates/pictura-render/src/document_ops/layer_ops/`:

- `clipping.rs` owns `base_below`, `clipping_base_allowed`,
  `release_run_above`, and `normalize_clipping`; the normalizer is called by
  hand from 13 production sites in `properties.rs` and `create.rs`.
- `properties.rs` owns `background_floor` (#717), the within-container move
  policy, Arrange, and the `deletable` last-layer guard.
- `create.rs` owns `insert_node`'s inline “no base at the bottom” rule and
  `has_background` (#717).
- `merge.rs` `splice_result`, the Stamp rollback, `smart_object.rs`
  conversion, `via.rs`, and `move_content.rs` splice containers without any
  normalization. `pictura-app`, `pictura-paint`, and scratch builders push
  layers directly.
- `Document.layers` and `Layer.children` are public `Vec<Layer>` fields, so any
  crate can bypass all of the above. The current tree has roughly 1,489
  `.layers` references, 195 `.children` references, 166 `doc.layers =`
  assignments, 96 `children` literals/assignments, 16 `&mut [Layer]` helper
  signatures, and 4 `&mut Vec<Layer>` signatures; most are reads or fixtures.

The Background floor and “one Background” rules landed in #717 and are
specified in `compositing/layer-management`. The clipping postconditions are in
`compositing/clipping-masks`. This change does not re-specify those commands; it
makes every structural edit honor them.

## Goals / Non-Goals

**Goals:**

- One owner for both `Document.layers` and `Layer.children`.
- Structural edits can only happen through atomic operations that re-establish
  the invariants before returning, with refusals leaving the tree unchanged.
- Reads and element writes stay as cheap and source-compatible as possible.
- A checkable invariant surface, so a missed mutator fails a test instead of
  shipping a bad stack.
- Keep malformed PSDs loadable as authored (#717's explicit non-goal).

**Non-Goals:**

- No document-level edit transaction (#739), recomposite, or history work.
- No compositor, codec, or PSD-serialization change.
- No private `Layer.clipping` / `Layer.background` fields; those remain a
  follow-up.
- No load-time normalization of vendor files, and no repair of a malformed
  Background placement.
- No generic closure-based stack editor.

## Decisions

### D1 — `LayerStack` lives in `pictura-core` and owns both tree levels

`Document.layers` and `Layer.children` become `LayerStack`, a newtype over
`Vec<Layer>` in `crates/pictura-core/src/layer_stack.rs`. Only `pictura-core`
can enforce cross-crate encapsulation: `pictura-render` is a separate crate, so
a render-local wrapper could not stop the app, codec, or paint crates from
splicing the public `Vec`.

The type derives `Debug`, `Clone`, `PartialEq`, and `Default`: `Document` and
`Layer` derive the first three, and `Document::new`/`Layer::default` construct
empty containers. Reads stay slice-like through `Deref<Target = [Layer]>`, plus
`Index<usize>` and `IntoIterator` for owned, `&`, and `&mut`. Element writes use
`IndexMut<usize>`, `get_mut`, `first_mut`, `last_mut`, and `iter_mut`. The
indexing impls are `usize`-only, so slice-level `swap`, `reverse`, and `sort`
are not reachable through indexing. `PartialEq` is implemented only between
`LayerStack` values; tests comparing to a `Vec` use `as_slice()`. There is
deliberately **no `DerefMut` impl at all**, so `push`, `insert`, `remove`,
`retain`, and `clear` no longer compile on the tree.

Alternative considered: a private field plus `layers()` accessors. Rejected:
it would churn the ~1,489 read sites for no additional structural protection.
The public field of an opaque type gives the same enforcement with a much
smaller diff. Alternative considered: a façade over the existing `Vec`.
Rejected: it cannot prevent a caller from going around it.

### D2 — Atomic primitives instead of a generic edit closure

The interface is a small set of path-based (`&[usize]`) edits. Every fallible
primitive returns `Result<_, StackError>` and MUST leave the tree unchanged on
`Err`; checks run before mutation.

| primitive | behavior / used by |
|---|---|
| `insert(parent, index, node) -> Result<(), StackError>` | new layer/group, duplicate, paste, via, lift, stamp; clears a requested clip with no legal base, refuses an insertion below the Background floor |
| `remove(path) -> Option<Layer>` | delete, stamp rollback; releases the run above a departing unclipped base |
| `move_node(from, to_parent, to_index) -> Result<(), StackError>` | move, arrange, drag/drop; releases the source run, refuses below the Background floor |
| `splice(removals, anchor_parent, anchor_index, replacement) -> Result<usize, StackError>` | merge; removes multiple arbitrary paths, releases runs above removed bases, inserts the replacement, returns its path |
| `replace(path, Vec<Layer>) -> Result<(), StackError>` | ungroup, smart-object conversion; single-removal shorthand over `splice` |
| `group(parent, indices, group: Layer) -> Result<usize, StackError>` | group selected layers, wrap one layer; fills `group.children` from the indices in stack order |
| `reverse(parent, start, end) -> bool` | Reverse |
| `truncate(parent, len) -> usize` | scratch clone scope; releases runs above removed bases |
| `retain_tree(parent, predicate) -> usize` | scratch adjustment filtering; recursive |
| `set_clipping(path, clipped) -> bool` | Create / Release Clipping Mask; `true` refuses an illegal position, `false` always succeeds |
| `promote_background(path) -> Result<(), StackError>` | Background From Layer; refuses a second Background |
| `well_formed() -> Result<(), StackViolation>` | validator |
| `from_layers(Vec<Layer>)` | raw install: codec, scratch, fixtures |

`StackError` carries `NoSuchPath`, `BackgroundFloor`, `SecondBackground`, and
`IllegalBackgroundNode`; primitives reject a node carrying the Background flag
unless it enters through `promote_background`.

A generic `edit(|tree| ...)` closure was rejected because a batch edit cannot
infer clipping intent from the final state alone. Removing `A` and `B` where
`A` is `B`'s clipping base is legal when both are reinserted into a new group
and illegal when they are deleted. The primitive knows which case it is; a
generic closure does not.

String-path parsing, locks, nesting locks, selection rules, the Background floor
clamp-vs-refuse distinction, and the last-layer guard stay in
`pictura-render::layer_ops`. The stack understands index paths and the tree
invariants, not Photoshop command policy. `remove` may empty a container
because merge and scratch documents need it; `delete_paths` keeps the last
top-level layer guard from `ui/layers-delete-guard`. `background_floor` and
`has_background` remain read-only render helpers over `&[Layer]`; they are not
deleted.

### D3 — Reconciliation is edit-aware, not a pure postcondition

`normalize_clipping` today uses `Moved { index, was_clipped,
same_base_below }` because a released run must stay released when reordered
back. That intent moves into `move_node`, which computes it internally from the
pre-move tree and also releases the contiguous run above the source when an
unclipped base departs. Both source and destination containers are normalized.

The other primitives have explicit intent:

- `remove`: release the contiguous clipped run above a departing unclipped
  base before the node leaves; then clear the container bottom and runs above a
  Background.
- `insert`: do **not** auto-join a run. An unclipped insertion between a base
  and its clipped layers becomes the run's new base; that is a legal positional
  re-base, not a base removal. A requested clip is dropped when the insertion
  point has no legal non-Background base, and an insertion below the Background
  floor is refused. This preserves current new-layer, via, lift, and Stamp
  behavior.
- `splice` / `replace`: release the runs above removed unclipped bases, insert
  the replacement at the anchor adjusted for the removals, and normalize every
  touched container.
- `group`: remove the selected indices without releasing their runs, fill
  `group.children` in stack order, normalize the group's children and the
  parent. Grouping a base, with or without its run, leaves the run attached to
  the group, which becomes its base (the documented #717 grouping behavior).
  Grouping a middle member makes the unclipped group the base for the members
  left above it; the resulting stack is valid.
- `reverse`: reverse the slice (flags move with their layers), then normalize.
- `truncate` / `retain_tree`: release the runs above removed unclipped bases and
  normalize the affected containers. `retain_tree` recurses into groups.
- `set_clipping(true)`: refuse when the node is at a container bottom or its
  nearest non-clipped base is a Background; `set_clipping(false)` always
  applies.
- `promote_background`: refuse when the tree already has a Background, remove
  the node (releasing its run), set the flag, clear its clip, insert at root
  index 0, and normalize both containers.

State validation cannot detect a silent re-base (the result still has a legal
base). The op-matrix test therefore checks more than `well_formed` for the
base-removal cases: a layer clipped before the edit whose base is gone must be
unclipped unless the operation explicitly reinserted it into a run, and a
run above an inserted layer is allowed to base on that layer.

### D4 — Raw install stays permissive; assertions are conditional

`LayerStack::from_layers(Vec<Layer>)` is the only raw install. It is used by the
codec, scratch documents, and test fixtures. It does not validate, normalize, or
reorder, so a vendor PSD with a clipped bottom or a Background-based clip loads
as authored and saves unchanged. History snapshots use `LayerStack::clone`, not
`from_layers`.

Every structural primitive runs `well_formed()` before and after the edit in
debug builds, but asserts only when the pre-edit tree was well-formed. This
avoids panicking on a malformed file whose untouched container is still
invalid; the first edit normalizes only the clipping in the containers it
touches, matching #717. A misplaced, grouped, or duplicate Background is left
as authored and reported by the check.

### D5 — Validator shape

`well_formed()` returns `Result<(), StackViolation>`. `StackViolation` derives
`Debug, Clone, PartialEq, Eq` and reports an index path plus a cause:

```
ClippedBottom(path)
ClippedWithoutBase(path)
BackgroundNotBottom(path)
BackgroundIsGroup(path)
MultipleBackgrounds(path)
```

The traversal is root-first, bottom-first within each container, and recurses
into groups. It is side-effect free, does not clone, and is O(nodes); it runs
only from debug assertions and tests, never in release edits.

### D6 — Verification strategy

1. **Validator first.** Add the free `well_formed(&[Layer])` form before the
   type change, so the op-matrix test can run against today's tree and answer
   the issue's open question about whether the un-normalized sites produce
   invalid stacks in practice.
2. **Op-matrix test.** A new `layer_ops/tests_stack.rs` applies every structural
   public op to a canonical tree, asserts `well_formed`, asserts the dry runs
   leave the document unchanged, and asserts base identity/released sets for the
   base-removal cases. This is the property test the issue asks for; it is
   separate from the existing per-op scenario tests, which remain the semantic
   oracle. Register it with `#[cfg(test)] mod tests_stack;`.
3. **Core property test.** A deterministic small-tree generator (fixed seed, no
   new dependency) applies the primitives and asserts the postcondition,
   including malformed input being either refused or normalized without panic.
4. **Codec test.** `malformed_clip_round_trips_unchanged` goes in a new
   `crates/pictura-codec/src/tests/clipping_invariants.rs` submodule, not the
   1,391-line `src/tests.rs`.
5. **No `proptest`/`quickcheck` dependency.**

### D7 — Migration mechanics

- 166 `doc.layers =` assignments and 96 `children` literals/assignments become
  `LayerStack::from_layers(...)`.
- Direct `push`/`insert`/`remove`/`pop`/`clear`/`swap`/`extend` calls become
  primitives, or `from_layers` when the call is constructing a fixture or
  scratch document. This includes `document_ops/mode/flat.rs` (`clear`),
  `mode/planes.rs` and `place_smart_object`/`mask_document` (`push`),
  `impl_layers_smart_object.rs` (`pop`), and history tests (`pop`/`swap`).
- 16 helpers that take `&mut [Layer]` and 4 that take `&mut Vec<Layer>` change
  to take `&mut LayerStack`; recursive calls pass the child stack. No
  `as_mut_slice` escape hatch is added.
- The duplicated `truncate_above`/`strip_adjustments` helpers in `pictura-app`
  and `pictura-paint` become `truncate`/`retain_tree` calls on the scratch
  tree.
- Direct `.clipping =`/`.background =` writes are audited; only the codec,
  history snapshot restore, and the flag commands remain.

### D8 — Residual bypasses and the guarantee boundary

The guarantee is "after a structural edit, a tree that was well-formed stays
well-formed". It is not "no caller can ever construct an invalid tree":
`from_layers`, whole-node replacement through `IndexMut`, and the public
`clipping`/`background` fields can still do so. The design keeps those hatches
because 1,489 read/element-write sites and the codec need them. A follow-up can
make the fields private and route every flag write through the stack.

## Risks / Trade-offs

- **Large mechanical diff (~300 sites)** → most edits are one-line
  `from_layers` conversions in tests and fixtures; audit with
  `rg '\.layers\s*=|children:'` and keep production behavior unchanged.
- **Intentional behavior changes at merge/smart-object/replace paths** → the
  spec already required released-run behavior for base removal; update any test
  that encoded the old silent re-base and call it out in the task result.
- **Insertion is behavior-preserving** → `insert` does not auto-join a run; an
  unclipped insertion becomes the new base. The existing
  `stamp_above_a_clip_run_keeps_its_placement` test must stay green.
- **`from_layers`/`IndexMut`/public flags remain** → explicitly documented
  residual bypass; the check and debug assertion cover structural edits.
- **Debug-only assertion** → release builds rely on the primitives themselves;
  the assertion is a net for tests, not the enforcement mechanism.
- **`&mut [Layer]` helpers** → change their signatures to `&mut LayerStack`;
  do not add a slice accessor that would expose `swap`/`reverse`.
- **File-size budgets** → `tests_stack.rs` is classified as code (1,200-line
  cap), not a test file; keep it under the cap or split it. The new codec test
  submodule is under `tests/` (1,400-line cap).

## Migration Plan

One change, ordered: validator and failing op matrix → type introduction and
mechanical migration → primitives and `layer_ops` rewrite → app/paint/codec
migration → full verification and archive. Rollback is a revert; there is no
on-disk or API-compatibility commitment beyond the internal crates.
