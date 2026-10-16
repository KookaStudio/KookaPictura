# Design: mask-paint-performance

## Context

See `proposal.md`. The composite path already reads a mask as one coverage byte
(`composite_rows.rs:99,114`; `composite.rs:1160-1173`). The edit path is the
outlier: `mask_document`/`gray_document` build an RGBA stand-in
(`mask_edit.rs:26,52`), `begin_mask_paint` seeds no acceleration
(`impl_paint.rs:518-526`), and `mask_dab` copies channel 0 back per dab
(`:558`).

## Decision (ADHD synthesis)

Four API shapes were explored under parallel frames (hardware, remove-the-
assumption, speedrunner, 3am-on-call, hostile-competitor) and the top three were
deepened — a dedicated grayscale engine on `&mut [u8]`, a stride-parameterized
shared kernel (`Stroke<Pixel>`), and GPU-R8 + mask preview. The synthesis:

- **Ship the smallest correct seam first: a 1-channel mask document.** The
  existing `Stroke` already tolerates *missing* channels — `plane_index`
  (`stroke.rs:1035`) yields `[Some(0), None, None, None]`, `write_pixel`
  (`:1065`) writes only channels it finds, and `Stencil::composite`
  (`:879`) defaults a missing alpha to 255. So a document whose single layer
  holds only channel `0` (the coverage) makes every dab write **one byte per
  pixel** with no engine change and no RGBA stand-in. This is exactly the
  "catch the glitch" path: no new type, no duplication, and the mask's channel 0
  after a dab is byte-identical to the RGBA document's channel 0 (both are
  `round(src*a + dst*(1-a))` with `src` the luma and `src_a` the coverage-times-
  opacity).
- **Do not add a `grayscale: bool` to `Stroke`** — a runtime flag is honored in
  one blend branch and silently dropped in others; the channel-missing data model
  is already mode-agnostic.
- **Defer the generic `Stroke<Pixel>`/stride trait** until a second consumer
  (channels panel, quick-mask) exists: it risks refactoring the golden-tested
  RGBA kernel for one caller (YAGNI, and regression risk).
- **Acceleration parity next**: a mask stroke selects the exact CPU raster, a
  reduced-level preview, or a GPU stroke through the same `preview_level`
  policy the pixel path uses, honouring the raster budget.

## Decisions

### D1. The mask edit document is 1-channel

`gray_document(w, h, coverage, fill)` becomes a one-layer, one-channel
(`id == 0`) grayscale `Document` whose channel 0 is the coverage. `mask_document`
is unchanged in shape. `write_mask_back` copies the single `w*h` plane
(`mask_edit.rs:94-121` already checks `source.data.len() < w*h`; with a
1-channel source `len == w*h`).

*Why not the RGBA stand-in:* it allocates and writes `4 * w * h`, forces
`Saved` to allocate four planes (`stroke.rs:96`), and copies channel 0 back — 4x
traffic for a 1-channel model the compositor already assumes.

### D2. Mask strokes pick the pixel path's acceleration

`begin_mask_paint` computes `preview_level(diameter, mask_w, mask_h)` and, over
the raster budget, seeds the same mechanisms the pixel path has. To keep the
exact oracle, the committed mask is produced by the exact CPU `Stroke` (a GPU or
preview stroke replays its sample log through the exact stroke at commit, exactly
as `end_paint` does for a pixel preview, `impl_paint.rs:856-868`).

- **Preview** presents a reduced level, not the document: the mask plane is
  rasterized at level N and the changed region is composited once per frame
  (already the present policy). The mask pyramid already exists for solo
  (`impl_core/display.rs`).
- **GPU** reuses `GpuStroke` with a channel count: the mask seeds as its coverage
  (1 plane), the dab uniform carries `channels == 1`, and the readback returns one
  plane patched straight into `LayerMask.data`.

### D3. Present the changed region only

The per-frame present stays the dab's dirty region (`mask_dab` already offsets it
by `mask.rect`, `impl_paint.rs:560-567`). No path rebuilds the full mask document,
the full pyramid, or the full document composite per dab. The commit does one
region-bounded exact write plus one region composite (`impl_paint.rs:823-848`).

## Risks / Trade-offs

- **Channel convention.** A grayscale `Document` layer must name its coverage
  channel `0` so `plane_index` resolves it; a test pins `mask_document` → dab →
  `write_mask_back` round-trip on the 1-channel form and its byte-equality with
  the RGBA oracle.
- **GPU parity.** A 1-channel GPU path must not round-trip through RGBA (banding).
  The `gpu_stroke_matches_the_cpu_oracle` pattern is extended with a mask variant
  asserting ±1 LSB against the exact CPU raster.
- **Scope creep.** Only the raster mask paint path changes; fills/filters keep
  their document-based editors via the same 1-channel `mask_document`, so they
  get the memory win for free but no new acceleration.

### D4. A live mask stroke counts as painting

The shell's `regionBlitted` slot only schedules the deferred `flush_present`
while `PictureView::is_painting()` (`frame.cpp:450`) — and `is_painting` checked
the pixel stroke only (`impl_paint.rs:996`). So a mask drag presented its
frame-opening dab and then accumulated the rest until commit. `is_painting` now
also counts `mask_stroke`, which restores the frame-by-frame present for both the
exact and GPU mask paths and stops the per-blit title/registry refresh during a
mask drag.

## Measurements

`cargo test --release -p pictura_app --lib mask_dab_profile_4000 -- --ignored --nocapture`
(4000² mask, ⌀512 brush, 16 dabs, exact CPU raster):

| path | per-dab |
|---|---|
| RGBA mask document (before) | 9.06 ms |
| single-channel document (after) | 7.55 ms |

The raster win is ~17% because the shared `Stencil::composite` still evaluates
the 4-channel blend and only the store width shrinks; the larger wins are (a) the
4x memory cut (the mask document and `Saved` carry one plane, not four) and (b)
the GPU path, which a mask brush over the raster budget now takes — a ⌀512 brush
on a 4000² mask (`514² = 264 196 > 262 144`) selects level 3 and the GPU stroke.
Small brushes (⌀ ≲ 150) stay on the exact CPU path and are sub-millisecond.
`gpu_mask_stroke_matches_the_cpu_oracle` (`#[ignore]`, run with a GPU) pins the
GPU path to the CPU raster within ±1 LSB.

## Migration Plan

None: view-only performance change, no persisted state, no schema change.
