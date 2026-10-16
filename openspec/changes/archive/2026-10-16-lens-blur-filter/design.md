# Design: lens-blur-filter

## Context

See `proposal.md` — Why. The engine applies a filter to a planar 8-bit
`PixelBuffer` and never sees the document (`pictura-filters/src/filter/apply.rs`),
while the app owns the document, selection, layer masks, and the alpha channel
(`crates/pictura-app/src/cxxqt_object/`). Blur kernels already share the
`blur.rs` helpers (`Bilinear`, `box_blur_planes`, clamp-to-edge) and the
`photorust::add_noise` seeded-noise model. The app carries non-scalar filter
data as a numeric id into a process-lifetime store (`filter_tools/displace_maps.rs`),
and resolves kinds from `f64` slots (`filter_map.rs`). Previews run through
`apply_filter_active_region` with a per-filter apron
(`pictura-render/src/filter.rs`); positional filters opt out of section
previews through `filter_preview_needs_whole_layer`.

## Goals / Non-Goals

**Goals:**

- A depth-driven, iris-shaped blur whose behavior matches `FILT-010`'s
  documented model (CoC ramp, bokeh shape, specular bloom, re-injected grain).
- One deterministic, validated engine kernel, no new dependencies, CPU-only.
- A CS6 dialog on the existing shared preview pane, with the three depth
  sources the app can actually materialize.

**Non-Goals:**

- Adobe pixel parity (the kernel is closed); this is a marked approximation.
- The Blur Gallery (Field/Iris/Tilt-Shift) — a separate capability.
- GPU acceleration; named document alpha channels in the source combo;
  interactive click-to-focus on the canvas preview (the shared dialog has no
  per-kind click hook).

## Decisions

### Depth map and the uniform case

`DepthMap` is a one-plane 8-bit map at the crate root, shaped like
`DisplaceMap` (`new` validating `width * height`, `width`/`height`, a private
sample accessor). `Filter::LensBlur.depth` is `Option<Arc<DepthMap>>`, and
`None` acts as a uniform depth of 255 (far), so a dialog opened on Source
`None` produces a uniform aperture blur and the "every kind defaults from an
empty slot list" invariant holds without a document. The alternative — a
`DepthSource` enum in the engine — was rejected because `apply` has no
document access; the app must materialize the plane.

### Kernel

- **CoC ramp.** `r = radius × |d − focal| / 255`, clamped to `radius` and
  rounded; `r == 0` copies the source pixel. This is the docs' `|d − focal|`
  ramp: focal 0 with a black-to-white map keeps the black side sharp and
  blurs the white side increasingly (parity criterion 7).
- **Iris mask.** For each integer radius used, precompute the integer-offset
  tap list inside the aperture, each tap weighted by a one-pixel antialiased
  edge. The boundary is the polygon of `shape.blades()`, whose apothem-normalized
  radius at angle θ is `cos(π/N) / cos(θ' − π/N)` with `θ'` the angle folded
  into one blade sector after `rotation`; `curvature` blends that radius toward
  1 (a circle). The tables are built once per apply, up to the master `radius`
  (`Σ πr² ≈ (π/3)·R³` taps, ~6 MB at R = 100), and shared read-only across
  rayon row tasks. A disc is the `curvature = 100` case, not a separate path.
- **Specular.** The gather sums the aperture-weighted color and a second sum
  over taps whose luminance is at or above `specular_threshold`; the result is
  `base + highlight × specular_brightness/100`, clamped to 255. CS6's exact
  compositing formula is closed, so this is a monotone approximation (marked
  `ponytail:`), enough to make an out-of-focus bright spot bloom.
- **Noise.** Reuse the photorust noise model: uniform draws or three summed
  hashes for a near-Gaussian bell, magnitude `amount/100 × 127.5`, one draw
  per pixel when `monochromatic`, from a `ChaCha8Rng` seeded by `seed`. The
  kernel writes color planes only; alpha is untouched and the render layer's
  alpha pass reruns the kernel on the grey alpha copy because Lens Blur joins
  `filter_preserves_opacity`'s moving list.
- **Mismatched maps.** A depth map whose size differs from the buffer is read
  bilinearly with a clamped normalized stretch (the `stretched` pattern from
  `photorust/displace.rs`), so the dialog proxy and Last Filter on another
  layer still sample the whole map.
- **Faster / More Accurate.** `faster` halves the tap density (every second
  row and column of the aperture footprint) and is a preview-only knob; the
  commit path normalizes it away (below). `preview_apron(LensBlur)` returns
  the master `radius`, the largest possible CoC, and `lens-blur` joins
  `filter_preview_needs_whole_layer` because the depth map is positional.

### Carrying the depth source

The engine cannot resolve a document source, so the app materializes it:

1. `filter_tools/lens_blur_depths.rs` keeps a `DisplaceMap`-style store
   (ids never 0, the eight most recent maps).
2. `filter_lens_blur_depth(source: i32) -> i32` builds the plane from the
   active layer — Transparency from channel `-1`, Layer Mask from
   `Layer.mask` (default-color filled), Selection from `selection_to_mask` —
   crops it to the active layer's rect, registers it, and returns its id.
   Source None returns 0; an unavailable source returns 0 as well.
3. The dialog resolves the source slot into the hidden id slot through that
   bridge whenever it reads its values, so every preview and the commit carry
   a fresh map (the selection or mask cannot go stale while the modal dialog
   is open).
4. `filter_from_kind_params("lens-blur", ..)` maps Source None to
   `depth: None` and any other source to `lens_blur_depth(id)?`, so id 0 with
   a real source refuses the filter — `apply_filter_active_region` sets
   `filter_error`, and the commit leaves the pixels unchanged with a message.
5. Last Filter replays the stored slots (and id) without reopening the
   dialog; Last Filter Settings reopens it prefilled and resolves again.
   Eviction after eight further map registrations refuses the replay, the
   same ceiling Displace documents.

### App wiring

- The `lens-blur` kind has 15 slots in dialog order: Preview, Source, Blur
  Focal Distance, Invert, Shape, Radius, Blade Curvature, Rotation, Specular
  Brightness, Specular Threshold, Noise Amount, Distribution, Monochromatic,
  then the hidden depth-map id and seed. Defaults:
  `0, 0, 126, 0, 0, 50, 33, 191, 44, 93, 47, 1, 0, 0, 1` (the CS6 dialog
  values in #338; the docs table's earlier values were marked inferred).
- The row uses only existing controls (`radio`, `choice`, `slider`, `angle`,
  `check`, `hidden`) and the shared `FilterPreviewDialog`; no custom dialog is
  needed. The commit normalizes the Preview slot to More Accurate so "Faster
  must never be committed" (docs edge case) holds.
- The CS6 group boxes are flattened into the shared dialog's parameter
  columns; a `ponytail:` note records the layout ceiling.

### Verification

- `tests/oracle/mapping.rs` gains a Lens Blur row with `im: None`,
  tolerance 0, and a note; `tests/README.md` mirrors it. No
  `scripts/filter_oracle.py` operator is added.
- Property/known-value tests live with the kernel and in
  `tests/oracle/properties.rs`: in-focus sharpness, the ramp, the point-light
  iris support and its rotation, the specular monotonicity, seeded noise and
  alpha, the stretch, and validation-before-mutation.
- A Lens Blur entry in `tests/kernel_digests.rs` pins the bytes.
- `pictura-render/src/filter/tests.rs` grows a moving-kernel case; the
  `positional_kinds_preview_the_whole_layer` app test gains Lens Blur.
- `cpp/tests/tst_lens_blur_dialog.cpp` covers layout/defaults, the depth
  sources, the unavailable-source refusal, the Faster-commits-accurate rule,
  commit/cancel, and Last Filter; `tst_filter_menu` expects 15 slots.

## Risks / Trade-offs

- **Cost.** The gather is `O(Σ CoC area)` per pixel; a 100 px radius over a
  large layer is seconds on the CPU. → Faster halves the preview taps, the
  proxy preview runs at 320 px, and the commit is one pass; the `ponytail:`
  note names the ceiling.
- **Approximation.** The CoC ramp, the iris curvature mapping, the specular
  formula, and the noise distribution are inferred, not Adobe's closed
  kernels. → They are marked as such in the spec and design; tests pin
  qualitative behavior only.
- **Defaults.** The screenshot in #338 is the only CS6 evidence for the
  numeric defaults, and it supersedes the docs table's inferred values. →
  The docs table is updated in the same change under `TASK-ALLOWS-DOCS`.
- **No named channels.** The source combo is static. → A `ponytail:` ceiling;
  a dynamic combo needs a custom dialog.

## Open Questions

None blocking. The exact CS6 `None`-source semantics (uniform far depth here)
and the specular compositing constant remain unverified against Photoshop and
can be tuned without changing the spec's observable requirements.
