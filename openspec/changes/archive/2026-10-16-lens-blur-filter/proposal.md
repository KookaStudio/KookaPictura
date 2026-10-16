# Proposal: lens-blur-filter

## Why

Issue #338 (part of the remaining-filter audit on #167). CS6's
`Filter ▸ Blur ▸ Lens Blur` is a disabled stub: there is no engine kernel, no
`lens-blur` kind, and no dialog. The docs contract
(`docs/06-filters/blur-filters.md`, `FILT-010`) fixes the model — a grayscale
depth map (selection, alpha channel, or layer mask) drives a per-pixel blur
radius (`|d − focal|`), an iris-shaped aperture (blade count, curvature,
rotation) shapes the bokeh, specular highlights bloom, and out-of-focus grain
is re-injected. Adobe's kernel is closed, so parity is behavioral only.

## What Changes

- `pictura-filters` gains `DepthMap` (one 8-bit plane), `IrisShape` (triangle
  through octagon), and `Filter::LensBlur { depth, focal, invert, shape,
  radius, curvature, rotation, specular_brightness, specular_threshold,
  noise_amount, noise_distribution, monochromatic, faster, seed }`. The kernel
  lives in `blur/lens.rs`: a per-pixel circle-of-confusion radius, a
  precomputed antialiased iris mask, an additive specular highlight, and
  seeded uniform/Gaussian noise. Alpha stays with the render alpha pass (Lens
  Blur joins the alpha-moving allowlist) and `preview_apron` returns the
  master radius.
- `pictura-app` adds a depth-source store (`filter_tools/lens_blur_depths.rs`)
  and a `filter_lens_blur_depth(source)` bridge that materializes the active
  layer's transparency, its layer mask, or the document selection as a
  `DepthMap`; an unavailable source resolves to no map and refuses the commit.
- The `lens-blur` kind takes 15 slots in CS6 dialog order: Preview
  (Faster / More Accurate), Source, Blur Focal Distance, Invert, Shape,
  Radius, Blade Curvature, Rotation, Specular Brightness, Specular Threshold,
  Noise Amount, Noise Distribution, Monochromatic, and the depth-map id and
  noise seed carried without widgets. `Filter ▸ Blur ▸ Lens Blur…` opens CS6's
  dialog on the shared preview pane, previews the whole layer (the depth map
  is positional), and always commits the More Accurate path.
- Verification: Lens Blur is classified no-equivalent in the ImageMagick
  mapping (tolerance 0); engine property tests cover focus sharpness, the
  depth ramp, the point-light iris shape, the specular boost, seeded noise,
  and validation-before-mutation; a golden kernel digest pins the bytes; the
  render alpha/apron tests grow; and a new Qt Test `tst_lens_blur_dialog`
  covers the dialog, the depth sources, commit/cancel, and Last Filter.
- Docs: `docs/06-filters/blur-filters.md` pins its previously inferred Lens
  Blur defaults and ranges to the CS6 dialog values shown in #338, in a
  separate `TASK-ALLOWS-DOCS` commit.

## Capabilities

### New Capabilities

<!-- none: both changes are ADDED requirements on existing capabilities -->

### Modified Capabilities

- `imaging/blur-filters`: ADD the *Lens blur (depth-map aperture convolution)*
  requirement — the `Filter::LensBlur` contract, the CoC ramp, iris shape,
  specular highlights, seeded noise, parameter validation, depth-map
  alignment, and the no-equivalent oracle classification.
- `imaging/filter-app-ui`: ADD the *Lens Blur dialog and depth source*
  requirement — the slot order and defaults, the CS6 dialog, the depth-source
  bridge and its refusals, the whole-layer preview, the commit-time quality
  normalization, and Last Filter behavior.

## Impact

- `pictura-filters`: `lib.rs` (`DepthMap`, `IrisShape`), `filter/types.rs`,
  `filter/apply.rs`, new `blur/lens.rs`, `tests/oracle/mapping.rs`,
  `tests/kernel_digests.rs`, `tests/README.md`.
- `pictura-render`: `filter_preserves_opacity`, `preview_apron`.
- `pictura-app`: `filter_map.rs`, `filter_tools.rs`,
  `filter_tools/lens_blur_depths.rs`, `filter_commands.{h,cpp}`,
  `filter_preview_dialog.cpp`, `frame_menus_filter.cpp`, new
  `cpp/tests/tst_lens_blur_dialog.cpp` (registered in `tests/CMakeLists.txt`),
  `tst_filter_menu` slot-count expectations.
- No new dependencies. CPU-only; the GPU filter path falls back to the CPU
  kernel for every unported filter, so no GPU work is needed.
