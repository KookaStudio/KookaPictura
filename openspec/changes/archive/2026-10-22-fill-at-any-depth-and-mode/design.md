# Design

## Context

The engine's working model is 8-bit. A document's *visible* depth and colour
mode ride alongside it: `Document.source_depth` records the depth a file was
read at, and `Document.source_mode` records a colour mode that was converted
to an RGB working copy on read. A layer's pixels at that depth live in a
retained native store, `Layer.source_channels` (`SourceChannels`, a depth, a
rectangle, and one `Samples` plane per channel id); the layer's
`Layer.channels` are the 8-bit narrowing of that store.

Two readers of those pixels disagreed about which copy is authoritative:

- The CPU compositor reads the native store, **per channel**: for a document
  with a recorded source depth and no converted colour mode, and a store
  matching the layer's rectangle and the document's source depth, it takes the
  store's plane for the channel it wants and falls back to the layer's 8-bit
  channel when the store does not carry that channel
  (`pictura-render/src/composite_native.rs::native_unit`,
  `pictura-render/src/composite_rows.rs`).
- The PSD writer emits the native store only when *narrowing it reproduces the
  8-bit working copy*; otherwise it widens the working copy
  (`pictura-codec/src/write.rs::native_plane`).

So a tool that writes only the 8-bit channels is invisible on the display, and
on save the layer is written from the 8-bit copy — the fill lands at 8-bit
precision, the layer's native precision is flattened, and the merged composite
is recomposited from the stale store. See `proposal.md` for the symptom and
`specs/tools/fill-engine/spec.md` for the required behaviour.

Three further constraints shape the approach:

- `pictura-paint` depends on `pictura-core` only, so the rule that decides
  which copy is authoritative has to live at or below `pictura-core` to be
  shared with `pictura-render`.
- The 8-bit fill's bytes are pinned by the existing tests, so the 8-bit path
  cannot be re-expressed in a different arithmetic without re-pinning them.
- `docs/` is the behavioural contract. `docs/04-image-ops/32-bit-hdr.md` places
  the toolbox tools and the brush modes under their own 32-bit rules, and
  `docs/03-tools/gradient-and-paint-bucket.md` places the Gradient and the
  Paint Bucket out of reach in Bitmap and Indexed. The `docs/` corpus never
  restricted `Edit > Fill` to 8-bit RGB, and says of the 32-bit Edit menu
  commands "All (Fill, Stroke, Free Transform, Transform, …)".

## Goals / Non-Goals

**Goals:**

- A fill writes the pixels the compositor reads and the writer emits.
- A fill blends at the document's own depth, so a 16-bit fill's result is not
  the 8-bit widening.
- A Grayscale layer is filled in its own single colour plane, and a Gradient's
  ramp lies on the gray axis.
- The 8-bit fill's bytes do not move.
- The rule that picks the authoritative copy is stated once and applied per
  channel, the way the compositor applies it.

**Non-Goals:**

- The Paint Bucket and the Gradient keep their documented availability rules
  (out of reach in Bitmap and Indexed, and the Paint Bucket at 32 bits). Those
  rules are unenforced today and stay unenforced: that is issue #490. The
  engine's depth correctness is not the gate that enforces them.
- The Brush, the Eraser, the Art History Brush, the retouch strokes, the
  destructive filters and `Image > Adjustments` write only the 8-bit channels
  and stay invisible on a native-depth document. That is the class #490
  tracks, with #745 its GPU twin.
- The Paint Bucket's *flood* still matches its region against the 8-bit
  working copy, so the region's tolerance is 8-bit on a 16-bit document. The
  fill that follows is native.
- `Edit > Fill` on a *mask* still writes the 8-bit mask; the compositor's
  native mask path has the same shadowing, and the mask fill is a different
  feature.
- No loud-failure plumbing in the app. The remaining refusals (a group, an
  adjustment layer, locked pixels, nothing changed) are the ones Photoshop
  makes too, except the pixel-locked case, which
  `compositing/layer-locks` already requires to be surfaced.

## Decisions

### The authoritative-store rule moves to `pictura-core`, and it is per channel

`pictura-core` gains the predicate the compositor applies, and the fill
applies the same one to each channel it writes:

```rust
impl Document {
    /// The retained store that is authoritative for `layer`, and the plane for
    /// `id` in it.
    pub fn authoritative_plane(&self, layer: &Layer, id: i16) -> Option<&Samples>;
}
```

The store is authoritative when the document records a source depth of 16
bits, records no converted colour mode, and the layer's store matches the
layer's rectangle and the document's source depth. A channel is taken from the
store when the store carries a plane for it; otherwise it comes from the
layer's 8-bit channel — exactly the compositor's per-channel fallback.

*Alternatives.* A single `Option<&SourceChannels>` for the whole layer cannot
express "colour from the store, transparency from the 8-bit channel", and that
mixed case is the common one: `Image > Mode > 16 Bits/Channel` builds the
layer's store from the **colour** channels alone
(`pictura-render/src/document_ops/mode/planes.rs::ensure_stores`), so the
store has no transparency plane, while a PSD read retains one. The compositor
handles both; the fill must handle both the same way. Duplicating the
predicate in `pictura-paint` is smaller, but the two copies decide the same
question about the same document and a silent disagreement between them is
exactly the bug being fixed. A predicate in `pictura-render` was not an
option: `pictura-paint` cannot depend on it.

### The store is decided before the layer is borrowed mutably

`fill_layer` resolves the layer with `layer_at_mut(doc, …)`, a `&mut Layer`
borrowed from the document. Asking the document about its own layer while
that borrow is live is a borrow error, so the fill makes two passes: an
immutable pass that decides which channels the store owns, and then the
mutable pass that writes them. The decision is a small per-channel map, not a
reference, so nothing borrowed is held across the two passes.

*Alternatives.* A method taking `&mut Layer` and returning the store's planes
would need the document's fields and the layer's fields at once, which is the
same conflict.

### Two engines, not one generic engine

The 8-bit path keeps its `f32`, 0–255 arithmetic; the native path blends in the
unit `f64` domain at the store's depth. The two are shaped alike — the same
source resolution, the same selection and strength arithmetic, the same
transparency lock, the same dissolve noise — so everything that is not the
blend itself is shared.

*Alternatives.* One engine generic over `Sample` would express the rule once,
but its 8-bit instantiation would compute `(src/255) * sa` where today's code
computes `src * sa`, and the `f32`/`f64` difference would move the occasional
rounded byte. `pictura-adjust` faced the same choice and kept `apply` and
`apply_native` apart, with the native entry pinned to agree with the 8-bit one
on a `u8` store. The fill has no such cross-check to lean on, because the
native path never runs on a `u8` store, so the cheaper guarantee — leave the
8-bit code alone — is the one that holds.

### The native engine owns the store's planes while it fills them

`SourceChannels.planes` is a `Vec<(i16, Samples)>` and `Samples` is an enum of
`u8`, `u16` and `f32` vectors, so four simultaneous mutable typed views of the
colour and transparency planes are not directly available. The engine takes
the planes it needs out of the store (leaving empty vectors behind), works on
owned vectors, and puts them back, re-sorting by channel id so the store's
ordering invariant (`SourceChannels::new` sorts) holds.

*Alternatives.* Matching the enum per plane would need one arm per plane
combination; converting the planes to `f64` rows and back would allocate the
whole fill region several times over; splitting the plane vector by index
would need a `split_at_mut` chain per plane. Taking the planes out is a
`mem::take` per plane, keeps the fill allocation-free apart from the store's
own vectors, and lets the row loop chunk the planes exactly as the 8-bit path
chunks its `Plane<u8>` slices.

### The native blend runs in the unit domain, at 16 or 32 bits

The blend reads the destination samples as unit values, mixes the unit source
against them, and writes the result back through the store's own scale. This
is the domain `pictura-adjust`'s `apply_native` uses and the one
`pictura-core`'s `Sample` trait provides. The engine is generic over the
sample width, so a 16-bit store and a 32-bit store take the same pass.

*Alternatives.* Keeping the arithmetic in each store's own scale (0–65535 for
`u16`) would need the scale to be a parameter of every formula and would have
no sensible scale for a 32-bit store, whose values may exceed 1.0. The
compositor's own native read uses the same unit domain, so the fill and the
compositor agree on what a sample means.

### After a native fill the 8-bit channels are the store's narrowing

The fill re-narrows the store's planes into the layer's 8-bit channels after
it writes them, so the working copy and the retained store agree. That
agreement is what the writer's `native_plane` check looks for, so a save emits
the native samples instead of widening the working copy, and it is what
`compositing/native-depth-edit-preserve` already requires of a transform.

*Alternatives.* Leaving the working copy alone would break that agreement, and
the writer would then widen the whole plane — losing the native precision of
the pixels the fill did not touch. Narrowing only the filled region would
leave the two copies disagreeing wherever they already disagreed.

### A Grayscale layer is a one-plane shape, and a Gradient's ramp lies on the gray axis

The engine resolves the colour planes from the layer: channel ids 0, 1 and 2
for an RGB layer, channel id 0 alone for a Grayscale layer. A Grayscale source
colour is reduced to its Rec. 601 luma before the blend, and because a grey
source over a grey destination stays grey in every brush mode, the existing
per-pixel blend is reused unchanged: it is handed a grey triple for both the
source and the destination, and the red component is the result.

A Gradient is different: the ramp is sampled per pixel, so luma-ing the
sampled colour would give the luma of the interpolation, not the interpolation
of the luma — and `docs/03-tools/gradient-and-paint-bucket.md` says a
Grayscale document's gradient "has no color stops beyond the gray axis". So
the ramp's **end colours** are luma'd before the ramp is sampled, which is a
change in `pictura-paint/src/gradient.rs` rather than in the engine.

*Alternatives.* Converting the document to RGB to fill it and converting back
would lose the layer's single-plane shape and the gray alpha semantics.

### The Rec. 601 luma lives in `pictura-core`

`pictura-core` gains `rec601_luma(r, g, b) -> u8`, the shared model crate with
no dependencies, and the fill engine uses it. The app's `helpers::luma_u8`
delegates to it, which removes a copy; `pictura-render`'s integer `luma_u8`
stays, because it has a different shape and a different caller.

### The fill is refused only where Photoshop refuses it

A group, an adjustment layer, locked pixels, a transparency lock with Clear, a
short selection, an empty paint and a zero opacity all refuse, exactly as
today. There is no refusal on the store's account: a store that lacks a
channel is not a reason to refuse, because the compositor falls back for that
channel too, so the fill falls back the same way.

### No app-side change

`edit_fill` already reaches the engine, the region refresh already recomposites
the region from the layer, and the save already refreshes the native composite.
Once the engine writes the copy the compositor reads, the display and the file
both follow. The app's history carries the native store in the *metadata* half
of its state — `adopt_metadata` replaces a restored document's metadata with
the target state's — so an undo restores the store as well as the 8-bit
channels.

## Risks / Trade-offs

- The 8-bit path's bytes move → the arithmetic is left untouched and the
  existing fill tests are the guard; a new scenario pins them. The existing
  tests pin behaviour rather than every byte, so the change also adds a
  byte-exact case for the 8-bit fill.
- The 8-bit channels are re-narrowed over the whole layer, so an 8-bit-only
  edit made earlier by another tool is overwritten by the store's content →
  that edit was already invisible, and the store is the authoritative copy.
- The fill's per-pixel blend is at 16 bits while the 8-bit path's is at 8, so
  the two disagree by up to one 8-bit step on a partly covered pixel → that is
  the point of the change; the 8-bit path is unchanged for 8-bit documents.
- The native engine writes the store's colour planes but leaves the store's
  transparency plane alone when the store has none, so the layer's 8-bit
  transparency stays the authority for alpha → that is the compositor's own
  fallback, and the two agree.
- A 32-bit destination sample above 1.0 is clamped into the unit domain on
  read, the same clamp the compositor's own native read applies → the fill
  agrees with what the display shows, and the filled region is replaced
  anyway.
- The Paint Bucket becomes effective at 32 bits, where `docs/` puts it out of
  reach → the rule's enforcement is #490's, and the engine's depth correctness
  is not that gate.
- The GPU compositor ignores the document's depth (#745) and composites the
  8-bit channels → after this change the 8-bit channels are the narrowing of
  the store, so the GPU shows the fill too, at 8-bit precision.
- A native store is deep-copied into every history state, and the app's fill
  helper clones the document before it edits it → the cost already exists for a
  document read at 16 bits; this change does not add stores.
- The engine's shape changes from "three colour planes" to "one or three" → the
  row loop carries optional G and B planes; the RGB shape keeps its arithmetic
  but not its code path.
