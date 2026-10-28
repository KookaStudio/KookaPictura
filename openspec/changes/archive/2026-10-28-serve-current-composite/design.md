# Design

## Context

See `proposal.md` for the motivation. The constraints that shape the approach:

- `Document::composite` is the maintained current picture. The edit transaction
  (`edit.rs::refresh_for`) patches it regionally after a bounded edit, stores a
  full composite after an unbounded one, and does neither for a content-neutral
  insertion or a metadata edit. Every pixel-changing committed edit and every
  presented preview therefore leaves it current.
- Two authoritative-document states leave it stale: a live paint or mask stroke
  (a layer stroke's presents patch only the regions already presented; a mask
  stroke never patches, so its cancel can restore from the untouched frame), and
  a lifted transform (lifting moves the selection's pixels into a floating layer
  and clears the source, and the session recomposites only when the *outgoing*
  session was lifted). Healing is not one of these — its pixels change on commit
  — and a non-lifted transform only trims the layer rect, which leaves the
  composite unchanged.
- `current_buffer` has a second role: it is the input to the repair paths that
  write the composite back (`composite_full`, `store_opened_composite`, the
  async apply jobs). It also defines the layerless behavior: a document with no
  layers answers with its embedded composite. It cannot start returning the
  cache.
- `PixelBuffer::data` is a refcounted, copy-on-write `Plane`. Cloning is an `Arc`
  bump; the first write to a shared plane copies it whole.

## Goals / Non-Goals

**Goals:**

- One accessor that answers "the picture as currently shown", so a caller never
  reaches `composite_active` for a frame the document already holds.
- The cached path costs no document-sized allocation.
- No observable behavior change: the accessor returns exactly what the existing
  full-composite path returns for the same state.

**Non-Goals:**

- Changing the compositor, the edit transaction, or the freshness guarantees.
- Making a live stroke or a lifted transform keep the composite current (both
  are behavior-changing designs).
- A benchmark suite; the win is structural (no composite). One ignored profile
  records the before/after for the PR.

## Decisions

### A second accessor, not a changed `current_buffer`

`current_buffer` stays "composite the stack, or the embedded composite when the
stack is empty". The new accessor is additive and falls back to it. Changing
`current_buffer` in place would make `composite_full` and the apply jobs read the
stale frame they are supposed to repair, turning every recomposite into a no-op.

### Return `Cow<PixelBuffer>`, borrowed from the document

`pub(super) fn picture<'a>(doc: &'a Document, view: &PictureViewRust) -> Cow<'a, PixelBuffer>`

- `Cow::Borrowed` is the served composite: zero bytes, and the type itself
  reports which state was served. `Cow::Owned` is a fresh composite. This
  satisfies "say which state it served" without a parallel enum or a metrics
  hook.
- Borrowing makes the compiler enforce the important invariant: a caller cannot
  write the document while it holds the picture. A cloned buffer would silently
  turn the next region patch into a full-document copy-on-write (64 MB at
  4000²), moving the cost instead of removing it.
- Alternative rejected: returning `PixelBuffer` (an `Arc` bump). Cheap, but it
  hides the copy-on-write hazard behind a `shares`-based convention and cannot
  express "you must drop this before editing".
- Call sites that need an owned buffer call `into_owned()`; for a borrowed frame
  that costs a document-sized copy — the same cost today's `composite_active`
  output already pays — so it is never a regression.

### Five guards decide freshness

The accessor serves the composite only when all hold; otherwise it falls back to
`current_buffer`:

| Guard | Why the composite cannot stand in |
| --- | --- |
| no live paint or mask stroke | a layer stroke lags its unpresented regions; a mask stroke never patches |
| no lifted transform | the lift rewrote the layer stack and the session did not recomposite |
| `doc.mode == ColorMode::Rgb` | a document converted to a non-RGB working mode (e.g. Grayscale) keeps its own planes, not RGBA |
| `composite.channels == 4` | a converted or read document can keep a 3-plane composite (Indexed, Bitmap, a retained 16/32-bit read) |
| `doc.source_depth.is_none()` | saving a 16/32-bit document rewrites `doc.composite` from the native samples, which is not the 8-bit composite these consumers read |
| composite dimensions equal the document's | a geometry change leaves the old-size composite |

Opened CMYK and Lab files are normalized to `doc.mode == Rgb` with `source_mode`
recording the header mode, so they are caught by the channel guard. An opened
Grayscale file keeps `doc.mode == Grayscale` and is caught by the mode guard,
which also covers documents converted to a non-RGB working mode. The two are kept separate because a future 4-plane non-RGBA composite
(a working CMYK mode) would pass the channel guard.

Consequence: a document whose composite is not four channels — an opened Indexed
or Bitmap file, a converted Grayscale document (one plane), a 16/32-bit file with
a retained 3-plane composite — does not hit the fast path until an edit rewrites
the composite through `store_composite`. That is accepted; it stays correct and can
be widened later if a profile shows it matters.

### The fallback is `current_buffer`

When a guard fails, the accessor returns `current_buffer(doc, view.gpu_compute)`.
That preserves the layerless rule (embedded composite, not an empty stack) and
the exact buffer every migrated caller receives today.

### Save and PDF export keep `current_buffer`

Both refresh the retained native composite before reading the picture:
`pictura_render::refresh_native_composite` rewrites `doc.composite` from the
native samples (`composite_native.rs`), so a picture taken after it would be the
native narrowing, not the buffer these paths encode today. They are not
interactive paths, so they stay on `current_buffer` rather than paying a copy to
take the picture before the refresh. The raster exports and the Save As format
encoders, which do not refresh native samples, move.

### The stroke query lives on the view

The QObject's `is_painting` becomes `PictureViewRust::is_painting` (stroke or
mask stroke live), with the QObject delegating. The accessor takes `&Document`
and `&PictureViewRust` rather than a bare flag so the freshness state cannot be
supplied by hand. The document must be the view's document (or an equivalent
clone, as `fills::apply` passes); the signature does not prove that.

The lifted-transform check reads the live transform session directly
(`live.transform()` with `lifted.is_some()`), not `modal_transform_blocks_edit`,
because the latter excludes the Move tool's controls box.

### Migration split

Only read-only consumers move. `current_buffer` stays for: `composite_full`,
`new_document`, `store_opened_composite`, the async apply jobs that call
`store_composite`, the transform previews (a replaced layer list), the
hidden-layer PDF background, `impl_core/save.rs` and `pdf_export.rs` (native
refresh, above), `displace_maps` (a separately decoded file with no view),
`level0_composited`, `document_to_image`, and the test-only `rebuild_display`.

`shrink` in `color_range.rs` and `refine.rs` takes `&PixelBuffer` so the accessor
does not force `into_owned`. (The two copies of `shrink` are identical; they are
left in place to keep this change scoped, and can be merged separately.)

### Verification

- The accessor's tests assert the `Cow` variant (borrowed vs owned) and equality
  with `current_buffer`, covering the edit, preview, history, layer-stroke,
  mask-stroke, lifted-transform, non-RGB, layerless, and size-mismatch cases.
  The stroke tests paint a dab and skip the present, so the maintained composite
  really is behind the served frame.
- A test-only counter in the accessor counts composites; the consumer test
  asserts a *delta* of zero (not an absolute value), so it is safe under a
  multi-threaded test runner. It drives the Sample All Layers sampling surface,
  which is reachable from Rust. GUI-only entry points (Magic Wand, Copy Merged)
  are covered by the accessor tests and the Qt Test layer, not by a new
  self-test check.

## Risks / Trade-offs

- [A caller keeps the picture alive across an edit] → `Cow` borrows the
  document, so this is a compile error rather than a hidden full-document copy;
  the fix is to scope the picture or `into_owned()` it.
- [A future edit path leaves the composite stale without a live stroke or lifted
  transform] → the spec states the freshness contract and the equality test
  fails; the new path must refresh the composite or composite before returning.
- [A consumer assumes RGBA and a non-RGBA composite leaks through] → the mode
  and channel guards are part of the accessor, not the call sites, so a new
  caller cannot opt out.
- [A borrowed picture blocks a needed mutation] → `into_owned()` is the escape
  hatch and costs what the old code paid.

## Migration Plan

Mechanical, no data migration: add the module and the view query, move the
read-only call sites, keep the derived/repair and native-refresh sites on
`current_buffer`. Rollback is reverting the module and the call-site edits; the
compositor and document model are untouched.
