# Design: Filter and adjustment parity bugs

## Approach

Build the pixel-distance fact into the same descriptor table `#826` made
exhaustive, then make every reduced copy use it. Restore the preview baseline
on refusal. Move adjustment math onto the document's sample range/gamma and
compute auto statistics over the selection coverage.

Order: #855 → #856 → #857 → #858 → #859 → #860 → #862.

## 1. #855 — pixel-distance fact and proxy scaling

### The fact

Add `pixel_distance: bool` to `FilterParam` in
`crates/pictura-app/src/cxxqt_object/filter_map/descriptor/mod.rs`. Every row
sets it explicitly; the C++ `measuresPixels` suffix heuristic is deleted.

The flag is set on every control whose value is a distance in document pixels:

- Already caught by the old suffix: Gaussian/Box/Shape/Surface/Lens Blur
  Radius, Motion Blur Distance, Smart Sharpen Radius, Unsharp Mask Radius,
  High Pass/Maximum/Minimum Radius, Offset Horizontal/Vertical, Dust &
  Scratches/Median Radius, Field/Iris/Tilt-Shift Blur, Path Blur Start/End
  Speed.
- Previously missed: Smart Blur Radius; Mosaic, Crystallize, and Pointillize
  Cell Size; Color Halftone Max Radius; Mosaic Tiles Tile Size and Grout Width;
  Stained Glass Cell Size and Border Thickness; Patchwork Square Size;
  Craquelure Crack Spacing; Emboss Height; Extrude Size/Depth; Glowing Edges
  Edge Width; Smart Sharpen Shadow/Highlight Radius; Colored Pencil Pencil
  Width; Paint Daubs Brush Size; Palette Knife Stroke Size; Underpainting
  Brush Size; Rough Pastels Stroke Length; Accented Edges Edge Width; Angled
  Strokes/Crosshatch/Ink Outlines Stroke Length; Spatter Spray Radius;
  Sprayed Strokes Stroke Length/Spray Radius; Sumi-e Stroke Width; Charcoal
  Thickness; Graphic Pen Stroke Length; Halftone Pattern Size; Water Paper
  Fiber Length; Wave Wavelength and Amplitude Min/Max.

Controls that are percentages, counts, angles, abstract level strengths (0–10,
1–15, and similar), or normalized fractions stay unflagged. The fact follows
the dialog's unit, not the kernel's calibration: a spatial size is flagged even
when the kernel maps it through a step or offset, because the value still means
document pixels. Dry Brush, Fresco, and Sponge Brush Size, Smudge Stick Stroke
Length, Poster Edges Edge Thickness, Watercolor Brush Detail, Plastic Wrap
Smoothness, Sponge Smoothness, Neon Glow Glow Size, and Ocean Ripple Ripple
Size/Magnitude are levels, not distances, and stay unflagged. A future audit
can add rows; the fact is the single place to do it.

### Sharing the fact

- `filter_descriptors()` copies `param.pixel_distance` into
  `FilterParamRow.measures_pixels`; `paramFromRow` copies it into
  `FilterParamSpec::measuresPixels`.
- `FilterParamControls::scaledValues` scales `spec.measuresPixels` rows instead
  of calling `measuresPixels(spec)`.
- `descriptor::scale_pixel_params(kind, params, scale)` in Rust applies the same
  rule: use the descriptor defaults when `params` is empty, multiply the first
  slot of each flagged control, round when `decimals == 0`, and clamp to the
  control's range.
- `filter_gallery_preview` calls `scale_pixel_params` per stack entry before
  resolving the filters; the commit paths (`filter_commit_start`/
  `filter_commit_finish`, `apply_filter_params`, the gallery's
  `filter_stack_commit_start`/`commit_filters`, and the older unused
  `apply_filter_stack`) never scale.
- `filter_thumbnail` gains a `scale` argument and calls `scale_pixel_params`
  with the parameter list it was given (empty means the descriptor defaults).
  The gallery passes the document-to-thumbnail cover scale and an empty list;
  the dialog thumbnail callers pass their own values and their proxy scale
  (1.0 where every control is normalized or a percentage). The gallery's
  catalog thumbnail is reduced in two steps: the whole document is proxied at
  `min(1, 160 / min(width, height))` (short side 160 px), then
  `thumbnailSample` cover-scales that proxy to 80×56 and centre-crops. The
  effective document-to-thumbnail scale is
  `max(80 / doc.width, 56 / doc.height)`; the gallery stores that scale and
  passes it to `filter_thumbnail` with the empty list, so the descriptor
  defaults are scaled against the thumbnail's real document footprint.

The dialog keeps its existing split: `scaledValues(scale)` feeds the reduced
`filter_canvas_proxy`, while `values()` feeds the full-resolution section and
exact previews.

## 2. #856 — whole-layer escalation

Set `whole_layer: true` on `radial-blur` (`descriptor/blur.rs`), `clouds`,
`difference-clouds`, and `fibers` (`descriptor/render.rs`).
`filter_preview_needs_whole_layer` already reads the descriptor, so the
single-filter section preview and the stack-preview path
(`filter_stack_preview`) pick them up. The shared dialog's proxy region
(`filter_kind_positional` → `proxyRegion`) reads the same flag, so a zoomed-out
Radial Blur, Clouds, Difference Clouds, or Fibers preview covers the whole
document rather than the visible crop. The Filter Gallery dialog's proxy
preview (`filter_gallery_preview`) deliberately stays a reduced copy of the
visible region, as `imaging/filter-gallery-preview` specifies; making it
whole-layer for these kinds would be a separate gallery requirement.

## 3. #857 — refused preview

`apply_op_active_region` already restores the baseline on error but returns
`None` without recompositing and after dropping the preview session. Fix the
callers that own the Qt object. Each wrapper cancels only the preview owned by
its own target (the active layer path, or the active mask path); a refusal
because another target owns the preview must leave that preview intact.

- `filter_preview` and `filter_preview_section`: on `None`, cancel this
  target's live preview (after a late refusal the core has already dropped it),
  recomposite, and return false.
- `filter_stack_preview`: the same wrapper around `run_stack`.
- `image_adjust_preview`, plus the Replace Color and Match Color preview and
  apply wrappers (including Match Color's malformed-statistics refusal before
  `run`): on a refused preview or commit, cancel this target's live preview
  and recomposite.
- `filter_commit_finish` on the worker's `Err` or a failed join:
  `commit_start` already restored the live layer, but the composite still shows
  the last preview; recomposite before returning false.
- `filter_commit_start` and `filter_stack_commit_start`'s synchronous refusals
  (before the worker starts) cancel this target's live preview and recomposite
  too.
- `FilterPreviewDialog::runPreview`: set `previewShown_ = shown` instead of
  only setting it true, so a refusal leaves the dialog's bookkeeping matching
  the restored document.

`apply_mask_op` is the exception to the first paragraph: it filters a clone and
writes back only on success, so on error it leaves a previously successful mask
preview live. Restore its baseline and clear the mask preview session there
before returning `None`; the preview wrapper's cancel also covers this path,
but a commit refusal must not leave the session open.

HDR Toning previews through the same core (`hdr_toning_preview` →
`apply_filter_obj_active_region`), but its dialog clamps every control to the
kernel's accepted range, so it has no reachable kernel-refusal state and needs
no wrapper change.

Recompositing on the rare refusal is cheaper than threading a second return
flag through every preview path.

## 4. #858 — Oil Paint and Offset

- Oil Paint: `descriptor/paint.rs` Shine becomes `minimum 0.0, maximum 10.0,
  value 5.5`. The engine and GPU already normalize `shine / 10`.
- Offset: add `repeat: bool` to `Filter::Offset`; `other::offset` takes it and
  clamps source coordinates when true (repeat wins over `wrap`). The descriptor
  gains a `Repeat Edge Pixels` checkbox after `Wrap Around` with value `0.0`;
  the slot order becomes H, V, Wrap, Repeat, Background RGB, Transparent, so
  `repeat` reads slot 3, Background moves to slots 4–6, and Transparent moves to
  slot 7. Horizontal and Vertical defaults become `0.0`. `mapping.rs`, the
  `Filter::Offset` destructure in `filter/apply.rs`, and every `Filter::Offset`
  literal gain `repeat: false` (except the tests that exercise the new mode).
  The alpha plane follows the same clamp because Offset is a moving kernel.
  Extend the zero/no-op test to cover `repeat` as well as wrap and transparent.
  The existing default assertion in `filter_from_kind("offset")`
  (`tests_impl.rs`) changes to `0/0` and the C++ Offset case in
  `pixelParametersScaleWithTheCopy` must set non-zero H/V before scaling,
  because the opening values are now zero.

Known docs/engine divergence: `docs/06-filters/other-filters.md` describes the
Offset H/V controls as unit values, while the engine, mapping and
`imaging/other-filters` use integer pixels. This change keeps the engine/spec
pixels; correcting the docs is a separate docs change. The issue's acceptance
only depends on the 0/0 identity and the edge mode.

## 5. #859 — Vibrance neutral gray

In both `color::vibrance` and `native::vibrance_native`, skip the vibrance
delta when the input saturation is zero:

```text
S' = if S == 0 { 0 } else { S + kv * (1 - S) * keep }
S' = clamp(S' * (1 + ks), 0, 1)
```

The `1 - S` falloff and skin damping stay as documented. The 8-bit and native
paths both get the guard. This changes the unconditional 8-bit Vibrance result
at `S == 0`; Auto's selection coverage changes its 8-bit result only when a
selection is active. `native_u8_equals_apply_for_every_covered_adjustment`
still pins the native u8 result to the changed 8-bit result, and the
ImageMagick oracle does not cover Vibrance. The `native-depth-adjustments` delta
modifies "The 8-bit application is unchanged" to name these two documented
exceptions while keeping the native-u8 parity contract.

## 6. #860 — Auto uses the selection

Add `pictura_adjust::apply_with_coverage(adjustment, buf, coverage:
Option<&[u8]>)`; `apply()` delegates with `None`. Only the Auto kernels read
the coverage: a pixel counts when its coverage byte is greater than zero. The
per-channel, joint, and neutral-midstone histograms/sums use the selected
population; the LUT is still applied to the whole working buffer, and the
existing write-back blends unselected pixels away.

`pictura_render::apply_op_region` builds a clip-sized coverage plane from the
selection coverage `LayerMask` (document coordinates) and passes it to the
adjustment call; entry `i` is the coverage at the clip's pixel `i`, so the
plane is clip-local. The coverage is built only when a mask is present. The
histogram totals are the selected pixel count, and three times that for the
joint histogram, not the whole clip count.

The preview section path still computes statistics from the section it
filtered; that is the existing section-preview trade-off, not a selection bug.

## 7. #862 — native-depth math

`apply_native` reads `samples.depth()` once:

- **Exposure**: a `F32` store is linear, so apply `c * 2^E + offset`, clamp at
  zero, and apply `gamma` directly; `U8`/`U16` keep the sRGB decode/encode. The
  existing `Sample` conversion clamps an `f32` input to `[0, 1]`; the output is
  stored through `f32::from_unit`, which leaves a result above `1.0`
  representable.
- **Posterize**: replace
  `round(round(u * denom) * 255 / denom) / 255` with
  `round(u * denom) / denom`. `levels == 255` stays the identity.
- **Black & White**: work in the unit domain. Clamp the grey to `[0, 1]`
  before the tint pass for every store; for a `U8` store round the grey to the
  8-bit grid first so the existing u8-parity test stays byte-identical;
  `U16`/`F32` skip that rounding.
- **Channel Mixer**: compute `(weights · rgb) / 100 + constant / 100` in the
  unit domain and clamp; no `round`/`/255` step. The u8 result matches the
  existing 8-bit kernel because `Sample::from_unit` rounds at 255.

Known docs divergence: `docs/04-image-ops/adjustments/posterize.md` generalizes
the 8-bit identity to `n = 65535` at 16-bit, while #862's acceptance requires
`levels == 255` to stay the identity (the current early return). This change
follows the issue; correcting the docs is separate. Likewise
`docs/04-image-ops/adjustments/exposure.md` treats a 32-bit linear sample as
possibly above 1.0, while the native capability's "sample conversion is total
and clamped" requirement clamps an `f32` input to `[0, 1]`; this change keeps
that input clamp and leaves an output above 1.0 representable, which satisfies
#862. Broader HDR input handling would need both documents changed first.

## Constraints and risks

- Follow `docs/dev/testing-conventions.md`: Qt Test for GUI, std `#[test]` for
  engine.
- Do not weaken or delete an existing test; update a baseline only with a
  before/after assertion.
- `filter_tools.rs` is close to the 1200-line cap; put the Rust scaling helper
  in `filter_map/descriptor/mod.rs` and new tests in test modules with room.
- `crates/pictura-adjust/src/tests.rs` is close to the 1400-line test cap; put
  the native/precision tests in `crates/pictura-adjust/tests/native_depth.rs`
  and the Vibrance test in a new integration test file.
- This branch starts from the open `#826` refactor (PR #879). Rebase onto
  master after that PR merges before opening the bundle PR.
