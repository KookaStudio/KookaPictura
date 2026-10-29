# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `pictura-render` already owns the row-parallel pattern: `composite_rows::for_each_row`
  splits a `Canvas` by rows, and a `#[cfg(test)] static FORCE_SERIAL` routes it
  through the serial branch so a test can compare both. Only `pictura-render`
  has that switch today; it is compiled out of every non-test build.
- `pictura-adjust` is a pure math crate with no `rayon` entry. Most of its
  kernels are three planar channels with independent pixels; the histogram,
  mean and coordinate-dependent kernels are the exceptions D1 calls out.
- `pictura-filters::blur::gaussian` rejects a one-channel buffer in `validate`;
  `pictura_filters::kernel::gaussian_blur_planes` (public) does not, and already
  processes each plane independently.
- `ByteTransform` wraps `lcms2::Transform<u8, u8>` with the default
  `AllowCache` flag. lcms2 only implements `Sync` for `DisallowCache`, so the
  current display transform cannot be shared across threads.
- `PixelBuffer::data` is a copy-on-write `Plane<u8>`. Cloning is a refcount
  bump; the first mutable write forks the whole allocation, including the alpha
  plane that the display conversion then overwrites nothing in.
- `rayon` is not in `[workspace.dependencies]`; each crate that uses it
  declares `rayon = "1"` itself.

## Goals / Non-Goals

**Goals:**

- Every per-pixel kernel on the four touched paths runs in parallel chunks.
- Each crate keeps a serial branch a test can select, with no cost in a
  production build.
- Byte-identical output, proven by tests rather than assumed.

**Non-Goals:**

- Parallelising the native-depth (`f32`) adjustment path, the destructive
  Convert-to-Profile path, or the neighbourhood blur itself.
- Moving the banded-compositing row threshold.
- Merging partial histograms or otherwise changing a reduction's sample order.

## Decisions

### D1 — Shared chunked helpers in `pictura-adjust`, not twenty rewrites

`common.rs` gains:

- `map_lut` / `map_float` keep their call sites and switch their inner loop to
  `par_chunks_mut`; `map_float`'s closure gains `Sync + Send`.
- A new `map_rgb(buf, n, f: impl Fn(u8, u8, u8) -> [u8; 3] + Sync + Send)`
  that splits the three planes, zips their chunked iterators, and calls `f` per
  pixel.

`map_rgb` covers Hue/Saturation, Vibrance, Color Balance, Black & White, Photo
Filter, Channel Mixer, Selective Color, Shadows/Highlights, Desaturate,
Threshold, Gradient Map and Color Lookup, because each already reduces to a pure
`(r, g, b) -> [r', g', b']`. `replace_color` derives `x`/`y` from the pixel
index, so it chunks by `width` (row-parallel) instead; `match_color` is
pointwise and uses `map_rgb` like the rest.

The histogram and mean *reductions* (`equalize`, `auto_joint`,
`snap_neutral_midtones`, `auto_per_channel`) stay serial: their output depends on
a reduction order, and merging partial histograms buys nothing on a single O(n)
pass. Their apply loops still run in parallel: `equalize` ends in `map_lut`, and
the `auto` kernels write their planes after the reduction.

Alternative rejected: parallelising only the two helpers the issue names.
`map_float` has one caller (`Invert`); the named 8–16× win lives in the colour
kernels that `map_rgb` covers.

### D2 — One matte-mask helper, reused by all eight layer effects

The mask multiply is duplicated verbatim in `satin`, `glows` (×2), `shadows`
(×2), `bevel`, `overlays` and `strokes`, differing only in the rect variable
names. It moves into one `layer_effects` helper taking the matte, its padded
width and origin, the clip, the layer and the document (for `mask_alpha`), then
runs `par_chunks_mut` by row. The `mask_refine::canvas(doc)` clip is hoisted out
of the loop.

The helper reads the existing `composite_rows::FORCE_SERIAL` static, so the
already-registered `parallel_composite_equals_sequential` test covers the new
path without a second switch.

### D3 — Blur the matte as one plane, not three

`blur_matte` builds a `channels = 1` `PixelBuffer` and calls
`pictura_filters::kernel::gaussian_blur_planes` directly with
`sigma_from_radius(size)`, rather than going through `blur::gaussian`. This is
exactly the arithmetic the three-plane call performed per plane, so plane 0 of
the old result and the new result are bit-identical; a test pins that over
random mattes and sizes. The `size <= 0` / non-finite guard stays.

Alternative rejected: teaching `pictura_filters::validate` to accept one
channel. That validation is shared by every filter, most of which assume three
or four planes.

### D4 — `par_iter_mut` for the clipping pass, not `for_each_row`

The clipping lerp walks `group.px`, `clipped.px` and `cover` in lockstep, and
the coverage collect walks `alone.px`. Neither is `Canvas`-row-scoped, so
`par_iter_mut().zip(...)` and `par_iter().map(...).collect()` are the direct
form; rayon preserves order on `collect`, so the coverage vector keeps its
pixel order. Both loops and the canvas-to-`u8` fill read
`composite_rows::FORCE_SERIAL`, so the render crate keeps one switch for every
parallel kernel it gains.

### D5 — Make the display transform reentrant, then chunk it

lcms2's `Transform::new_flags` is defined on the `AllowCache` impl and coerces
its flags back with `allow_cache()`, so it can never return a `Sync` transform.
The reentrant constructor is `Transform::new_flags_context(GlobalContext::new(),
…, Flags::NO_CACHE)`, which returns
`Transform<u8, u8, GlobalContext, DisallowCache>`. `ByteTransform`'s field type
changes to that (the only type that implements `Sync`), and its constructor
calls `new_flags_context`. `NO_CACHE` inhibits only lcms2's one-pixel memo; it
does not change the transform's arithmetic. With a `Sync` transform,
`convert_planes_with`:

1. interleaves the three source planes into `packed` in parallel chunks,
2. splits `packed`/`out_packed` into equal **whole-pixel** chunks (a multiple of
   3 bytes; lcms2 asserts a `u8` slice's length is a multiple of the pixel
   format's bytes-per-pixel) and calls `apply` per chunk in parallel,
3. builds the output `PixelBuffer` with `Plane::build`, writing the three
   transformed planes in parallel and copying the alpha plane only when
   `buf.channels == 4`, so the whole-buffer COW fork disappears.

`ByteTransform` has exactly one consumer, so the field change does not ripple.

Alternatives rejected:

- Per-worker transforms via `for_each_init`: reparses both profiles on every
  rayon worker, and the display path already caches one transform per calling
  thread.
- Keeping the transform serial and parallelising only the interleave: leaves
  the dominant cost serial.

### D6 — One test-only serial switch per crate, in unit tests

`pictura-adjust` and `pictura-codec` each get a `#[cfg(test)] static
FORCE_SERIAL`, mirroring `composite_rows`. The equality tests live in the crates'
unit-test modules (for example `src/tests.rs`, `src/icc/tests.rs`), not in
`tests/`: an integration test links the library built without `cfg(test)`, where
the switch does not exist.

### D7 — Declare `rayon` in `pictura-adjust`'s manifest

`pictura-filters`, `pictura-render` and `pictura-codec` each already carry
`rayon = "1"`. `pictura-adjust` follows the same pattern rather than
introducing `[workspace.dependencies]`, which no crate uses. The commit message
states the dependency was already in the tree (AGENTS.md rule 4).

## Risks / Trade-offs

- **Nested rayon in the banded path** (bands in parallel, kernels in parallel
  inside each band) → rayon work-steals nested `par_` calls, so this is safe;
  the inner chunk count is small relative to the band.
- **Rayon overhead on tiny buffers** (a few-row region, a 1-pixel probe) →
  accepted: the existing row helpers carry no threshold either, and the region
  paths this change targets are at least a few thousand pixels.
- **`NO_CACHE` slows a serial transform** by removing the 1-pixel memo →
  accepted; the only consumer is being parallelised, and photographic input
  rarely repeats a pixel. `byte_transform_cache_flag_does_not_change_output`
  pins cache/no-cache byte equality.
- **`FORCE_SERIAL` is process-global** → the existing tests already share it;
  nextest runs one test per process, which is the documented runner. New tests
  reuse the same statics rather than adding more.
- **File-size caps** → `layer_effects/mod.rs` is already 1084 LOC, so it stays
  under the 1200 hard cap with limited headroom; `icc.rs` gains little. No
  allowlist entry is needed.
- **A reduction silently parallelised** → the spec's byte-identity scenarios
  fail first; the design keeps every order-dependent kernel serial.

## Migration Plan

One branch, one PR. Rollback is reverting the commit: no on-disk format, API,
or document state changes.
