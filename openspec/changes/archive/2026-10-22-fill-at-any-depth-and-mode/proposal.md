# Fill a layer in the document's own colour mode and bit depth

## Why

`Edit > Fill` does nothing on a 16-bit RGB document and does nothing at all on
a Grayscale one (#791). The 16-bit case is the worse of the two: the fill
runs, marks the document dirty, records a `Fill` history state, and the
display does not change.

The cause is not the guard in `fill_layer` that issue #791 cites.
`Document::depth` is the *working* depth and is always `BitDepth::Eight` — the
codec sets it on read, `convert_bit_depth` never touches it, and the writer
refuses to save a document whose working depth is not 8 — so the depth arm of
that guard never fires. The cause is that a 16-bit layer has two
representations of its pixels and the engine reads the one the fill does not
write: the compositor reads the retained native store (`Layer.source_channels`),
while `fill_layer` writes the 8-bit working copy (`Layer.channels`). The writer
already resolves this the other way round — it emits the retained store only
when narrowing it reproduces the working copy, and otherwise widens the working
copy — so the compositor and the writer disagree about which copy is
authoritative, and a fill lands in neither the display nor the merged
composite. It is also silently lossy: the layer is written from the 8-bit
working copy, so the whole layer's native precision is flattened to 8 bits, not
just the filled pixels.

## What Changes

- Fill the pixels the compositor reads: the retained native store when the
  document has one that is authoritative for the layer, the 8-bit channels
  otherwise. The rule that decides this moves out of the compositor into
  `pictura-core` so the fill and the compositor cannot drift apart, and it is
  applied per channel, the way the compositor already applies it.
- Fill a 16- or 32-bit document as well as an 8-bit one.
- Blend at the store's own depth, so a 16- or 32-bit fill's result is not
  the 8-bit widening, and keep the 8-bit channels as the store's narrowing
  afterwards — the invariant the compositor and the writer both rely on.
- Fill a Grayscale document: a gray layer carries one colour plane, the
  source colour is reduced to its Rec. 601 luma, and a Gradient's ramp lies on
  the gray axis.
- Keep the 8-bit fill byte-identical to today. The native path is a separate
  engine, as `pictura-adjust` keeps `apply` and `apply_native` apart.
- No app-side change: the command already reaches the engine, so a fill that
  works needs no new plumbing.

## Capabilities

- Added: `tools/fill-engine` — the shared fill engine: the colour space a
  layer is filled in, the pixels a fill is written to, and the depth it is
  blended at.
- Modified: `ui/edit-menu-commands` — `Edit > Fill` and `Edit > Stroke` fill in
  any document colour mode, and in an 8-bit or 16-bit document.
- Modified: `color/bit-depth-sample-model` — a plane edited in the retained
  native store is re-encoded at the source depth rather than widened.

## Impact

- Code: `pictura-core` (`Document::authoritative_plane` and the shared Rec.
  601 luma); `pictura-render` (`composite_native::native_unit` calls the
  shared rule, a pure move); `pictura-paint` (`fill.rs` gains the
  one-or-three colour plane shape and the native dispatch, `gradient.rs` gains
  the gray ramp, a new `fill_native.rs` holds the native engine).
- Tests: `pictura-paint` unit tests, the `pictura-render` native-depth round
  trip, and `tst_fill_tools` at the app level.
- No new dependency. `docs/` is unchanged: the corpus never restricted
  `Edit > Fill` to 8-bit RGB, and its 32-bit Edit menu commands are all
  available, so no documented behaviour changes.
- `Edit > Fill`, `Edit > Stroke`, the Paint Bucket and the Gradient all change
  together; that is the point of the shared engine.
- **Out of scope, and left as they are.** The Brush, the Eraser, the Art History
  Brush, the retouch strokes, the destructive filters and `Image > Adjustments`
  write only the 8-bit channels and stay invisible on a native-depth document;
  that is the class #490 tracks, with #745 its GPU twin. The Paint Bucket's
  *flood* still matches its region at 8-bit precision, and `Edit > Fill` on a
  *mask* still writes the 8-bit mask. The Paint Bucket becomes effective at
  32 bits, where `docs/` puts it out of reach; the rule's enforcement is
  #490's, and the engine's depth correctness is not that gate.
