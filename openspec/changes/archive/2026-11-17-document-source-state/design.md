# Design

## Context

See `proposal.md` for the motivation and accepted scope from #1345. The user
confirmed two encapsulated values and unchanged pixel/save behavior.

`pictura-core/src/lib.rs` currently exposes six independent source/profile
optionals plus `composite` and `merged_composite_present`. Core's
`document_composite.rs` decides retention, render's `DocumentComposite` decides
storage/shape and maintains pixels, and render's mode module computes effective
mode/depth. `retains_source_depth` scans the layer tree as well as the document
store; `authoritative_plane` additionally checks source mode and the layer store.

The codec has important legal combinations beyond a simple loaded/new split:
`read/merged.rs` builds both files without merged data and opens that skip decoding
existing merged data. The former currently carries a zero-filled tiled placeholder
with output disabled; the latter can carry an empty buffer with output enabled.
The zero-filled placeholder still satisfies the current size/nonempty storage
predicate; this refactor must not silently change that answer.

Two app duplicates survive at `impl_transform/mod.rs` and `impl_history.rs`.
The issue's `filter_tools/imp.rs` duplicate has already been replaced by regional
and banded hidden-layer reads. History's `damage.rs`, plane extraction, and memory
accounting also depend on the fields and must migrate. Exploration used direct
source verification because the available graph index does not cover this worktree.

## Goals / Non-Goals

**Goals:** concentrate correlated decisions and validation in core, preserve
borrowed access and COW storage, and migrate all workspace consumers in one change.
Keep the source/composite seam small enough for consumers and tests to share it.

**Non-Goals:** change color math, codec encoding, freshness tracking, file-backed
tile ownership, layer storage, general document validation, or rendering policy.
Do not make every `Document` field private or introduce traits/dependencies.

## Decisions

### 1. Own two concrete state values in core

Add `document_source.rs` for `SourceState` and extend core's
`document_composite.rs` with `CompositeState`. Keep the remaining working mode,
working depth, canvas, layer tree, and compression fields in their current roles.
The two values can be public document members to preserve `..Default::default()`
construction; their correlated internals are private. Constructing a value must
cross its checked interface. Derive the existing clone/debug/equality semantics.

`SourceState` owns recorded mode/depth, retained document planes, palette,
converted-away ICC, and working ICC. Use a private representation with checked
construction/replacement rather than exposing the same six independently mutable
optionals behind setters. Retain optional provenance internally where it is real
data: recording a depth is independent of retaining native samples. A palette
must be paired with Indexed representation; a native sample variant must agree
with its declared depth (packed Bitmap samples remain bytes at depth One).

`CompositeState` owns the buffer and merged-image output policy. Keep an explicit
absent-buffer representation or the current empty buffer internally, but expose
absence through queries rather than asking callers to interpret a sentinel.
Preserve channel metadata and zero-sized default-document behavior when borrowing
the buffer. Zero-filled no-merged placeholders remain legal. Output enabled/disabled
is an independent choice within the value; it is not inferred from the buffer.

**Alternatives considered:** one enum for every source/profile/composite
combination would multiply independent dimensions and confuse provenance with
authority. A private wrapper with unrestricted mutable access to all old fields
would preserve the problem. Two checked concrete values localize the decisions
without introducing a generic state machine or compatibility shadow fields.

### 2. Validate representations without banning existing fallback states

Checked source construction rejects palette/mode contradictions and declared-depth
versus sample-variant contradictions. A high-depth retained document store must
agree with a recorded high depth when present. Check buffer-internal geometry on
replacement, with checked arithmetic; do not expose mutable references that can
change its dimensions/channel count separately from its data.

Do not treat a layer's stale retained rectangle, absent channel, absent document
store, recorded depth without retained samples, converted-mode store, or buffer
dimensions differing from the current canvas as a new load error. These already
have fallback/query semantics. Validation concerns each value's own representation;
queries compare it with live document/layer geometry. This avoids moving the whole
mutable canvas/layer model into this issue.

The construction test matrix will explicitly include:

| Source case | Effective mode/depth and authority to preserve |
|---|---|
| New/default and ordinary 8-bit RGB/Grayscale | Working mode, effective depth Eight, no retained native authority |
| Recorded 16/32-bit depth without samples | Recorded provenance survives, retained-depth false, effective depth Eight |
| 16/32-bit working-mode composite samples | Retained-depth true; matching layer stores can be authoritative |
| 16/32-bit descendant layer samples only | Same retained-depth result without requiring a document sample store; authority additionally requires matching recorded source depth and no converted source mode |
| Normalized CMYK/Lab | Recorded mode remains visible; retained source-mode planes never become working-RGB authority |
| Bitmap read/conversion | Effective depth One; preserve presence/absence of the existing packed store |
| Indexed read/conversion | Recorded Indexed mode plus palette; preserve optional retained indices |
| Duotone/Multichannel normalization | Preserve recorded mode and the codec's current retained representation |
| Incoming Convert/Preserve, later Assign/Convert | Original and working ICC are distinct and can coexist after later assignment |

For each source row, test stored/absent composite states wherever currently
supported, and both output-policy choices for layered documents. Use the existing
writer/oracle behavior as the compatibility reference, including current native
CMYK/Lab support; older comments describing only 8-bit retained Lab samples do not
justify dropping stores the present codec preserves.

### 3. One implementation per consumer decision

Expose effective mode/depth and stored-composite queries on `Document`, backed by
the state values and current layers/canvas. Keep `retains_source_depth` and
`authoritative_plane` as the core decision points and preserve their existing
fallback rules. Never cache a retained-depth boolean beside mutable layer stores.

Move the implementation of render's `document_color_mode`, `document_bit_depth`,
and `DocumentComposite::is_stored` behind those core queries; retain thin forwarding
functions where they avoid unnecessary downstream interface churn. Keep
`composite_is_authoritative` as the retention decision and `is_shape_valid` as the
existing working RGB/four-channel/dimensions check. Shape validity is not freshness.

The transform backdrop and named-snapshot restore checks call the same stored
predicate. Search all current callers rather than restoring the removed filter
check just to satisfy the issue's stale count. Add a render check proving its
forwarders agree with core for stored, absent, empty, and mismatched cases.

### 4. Borrow data; stage related replacements before publishing

Provide borrowed views of profile/palette/retained planes and the composite
buffer. For edits, expose samples/plane bytes with metadata fixed, or accept a
replacement value built before publication. Do not return unrestricted
`&mut SourcePlanes`, `&mut PixelBuffer`, or a mutable bag of source fields.
Existing row/tile writers must remain usable without dense materialization.

Codec read/normalize and render mode/depth conversions build a complete candidate
source value, then replace it. Geometry operations transform retained samples
into a candidate store with their new geometry. Profile operations update the
working profile with resource 1039 through the existing codec operation, retaining
the converted-away original profile when appropriate. Compound replacements
validate before changing either affected value, so refusal leaves the document
equal to its pre-transition clone. This does not add a new transaction framework.

Render's `DocumentComposite` remains the owner of render/patch/refresh algorithms;
core state owns storage validity and access. History needs an explicit detach/reattach transition through the interface: a
hollow composite retains its dimensions, channel metadata and output policy while
its plane is absent. Traversal retains the composite slot even while detached, so
its fixed tracked-plane order is unchanged. Reattaching validates the plane against
the retained metadata before publication and reuses its shared allocation. History
also compares working-profile bytes for display damage. Recovery, export, save, and
thumbnail readers borrow those same stores.

The application picture-save path prepares a composite from layers when needed
before invoking its writer. Direct codec writers retain their current buffer
preconditions and errors; an absent buffer alone does not make them render layers.

**Alternatives considered:** cloning pixel data for getters would defeat the
existing COW and large-document contracts; duplicating legacy fields for migration
would leave two authorities. Neither is needed for this workspace-wide refactor.

## Risks / Trade-offs

- Public field removal has a broad compile impact -> migrate struct literals,
  examples, tests, history plane walkers, and memory counters before the first build.
- Accidentally deriving output policy from storage -> preserve skipped-merged and
  no-merged fixture checks plus streamed output comparisons.
- Tight validation rejects an existing fallback -> test the legal matrix before
  implementing; keep cross-document/layer geometry checks in queries.
- A retained-depth flag becomes stale after layer edits -> keep the existing
  live tree scan in the document query.
- Encapsulation causes hidden allocations/materializations -> retain `Plane` and
  `Samples` storage and prove allocation sharing in existing clone checks.
- Older capability prose names removed fields -> this change updates the explicit
  public-field and clone contracts; other logical store references describe the
  same data accessed through the new interface, not promises of writable fields.

## Migration Plan

1. Write all new automated checks named in `test-plan.md` and adapt existing
   checks to the intended interface without changing their expected behavior.
2. Implement the core values and queries, migrate codec/render/app/history and all
   remaining workspace field access, then complete source-level inspection.
3. Compile only after all code is written, using the local CPU budget (at most
   80%). Fix compile errors and rebuild only after code changes; run the focused
   checks, existing independent oracles, and required workspace gates.
4. Require unchanged fixture/golden bytes and no public legacy field aliases.
   Before shipping, run the test-plan guard and archive this change, syncing its
   deltas as required by the repository's zero-open policy.

No persistent-format migration is needed. Reverting the implementation restores
the previous Rust interface without changing files saved by either version.
