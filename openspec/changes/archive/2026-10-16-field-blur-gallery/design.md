# Design: field-blur-gallery

## Context

`docs/06-filters/blur-gallery.md` (`FILT-092`) is the contract: CS6's Blur
Gallery is GPU/OpenCL-gated in Adobe, the shaders are closed, and the doc's
algorithm sketch is explicitly `(inferred)`. The engine applies a filter to a
planar 8-bit `PixelBuffer` with no document access
(`pictura-filters/src/filter/apply.rs`), and the app resolves kinds from
`f64` slots (`cxxqt_object/filter_map.rs`). Bespoke workspaces already exist
for Lighting Effects (`cpp/lighting_effects_dialog.{h,cpp}` +
`lighting_canvas.{h,cpp}`) and Lens Flare, both rendering their own preview
through `filter_thumbnail` and committing through `filter_commit_start`.

## Goals / Non-Goals

**Goals:**

- A deterministic, validated pin-interpolated variable blur (CPU, no new
  dependency) matching the doc's documented behavior: pins anywhere, per-pin
  amounts, a smooth gradient, pins outside the image anchoring the borders,
  a zero-pin no-op.
- The gallery's Blur Effects (Light Bokeh, Bokeh Color, Light Range) and the
  owner-requested Noise grain tab, both gated to the blurred pixels.
- A reusable Blur Gallery workspace shell: proxy preview, draggable circle
  pins with a blur handle, the Blur Tools panel, the Effects / Motion Effects
  / Noise tabs, `H` hide-UI, commit/cancel, Last Filter.
- Reuse across #341/#342: the canvas, panel frame, and commit path are
  effect-agnostic; only the per-effect controls and kernel differ.

**Non-Goals:**

- Adobe pixel parity (the kernel and effects curves are closed).
- The `M` mask view, Iris Blur and Tilt-Shift, Path/Spin Blur (post-CS6), and
  the Motion Effects controls (the tab states they are unimplemented).
- GPU acceleration; painting pins directly on the main document canvas.

## Decisions

### Pin model and field interpolation

`BlurPin { x: f64, y: f64, blur: f64 }` is at the crate root, exported like
`DepthMap`/`IrisShape`. `x`/`y` are fractions of the buffer (0..1 inside,
outside allowed); `blur` is in buffer pixels, capped by
`MAX_FIELD_RADIUS = 500` (the doc's unverified `~0..500 px`); at most
`MAX_FIELD_PINS = 16` pins. The per-pixel radius is inverse-square-distance
weighted: `r(p) = Σ wᵢ·blurᵢ / Σ wᵢ` with `wᵢ = 1/(dᵢ² + 1e-6)`. At a pin the
weight dominates so the local radius is that pin's amount; between pins the
field is a smooth weighted average; a far pin's influence falls with the
square of the distance. The doc names "inverse-distance / radial basis"; the
exponent and epsilon are a marked approximation (Adobe's field is closed).

### Kernel: variable radius via a box-blur level pyramid

A per-pixel disc gather (the Lens Blur approach) is `O(Σ CoC area)` per pixel
and cannot drive a live pin drag. Instead the kernel builds a level pyramid:
level 0 is the source, and level `k ≥ 1` is three box-blur passes of radius
`rₖ = round(2^(k/2))` over the source (three boxes approximate a Gaussian of
σ ≈ rₖ), with nominal radius `Rₖ = 3·rₖ` under the repo's 3σ convention
(`kernel.rs::sigma_from_radius`). Levels are built only up to the field's
maximum. For each pixel the two levels bracketing `r(p)` are blended
linearly: `out = (1−t)·Lₖ + t·Lₖ₊₁`. This is `O(n)` per level (the box pass
is a prefix sum, `blur.rs::box_blur_axis`), deterministic integer arithmetic,
and smooth in `r`. The pyramid is the approximation of the doc's inferred
disc/bokeh kernel, marked `ponytail:`; highlights blur softly rather than
staying disc-shaped.

The kernel writes the colour planes only (`min(channels, 3)`); the render
layer reruns it on the grey alpha copy because Field Blur joins
`filter_preserves_opacity`'s moving list, so an unlocked layer's edge smears
as CS6's does. `preview_apron` returns the largest pin radius. The alpha pass
drops the colour-only effects through `alpha_kernel`.

### Blur Effects and Noise passes

The field pass records a per-pixel weight (`r(p)/max_pin`, 0 sharp to 255 at
the largest pin) beside the bracket map, and one interleaved-pixel pass runs
the two effects over it, so sharp pixels never change.

- **Bokeh.** When enabled, a pixel whose luminance lies in `light_range`
  is multiplied by `1 + 2 × light_bokeh/100 × luma/255 × weight` (Light
  Bokeh: a bright, well-blurred pixel blooms brilliantly toward white, as the
  reference shows), then — unless the lifted pixel is blown past ~250 — has
  its channels pushed away from their luminance by
  `1 + bokeh_color/100 × weight` (Bokeh Color). Both curves are inferred
  approximations of the doc's `(inferred)` description.
- **Noise.** When enabled, grain of `amount/100 × 127.5 × weight` is added,
  damped in bright areas by `1 − highlights/100 × luma/255`. `Gaussian` draws
  a near-Gaussian per-pixel sample (three summed uniform hashes), `Uniform` a
  uniform per-pixel sample, and `Grain` a bilinear lattice of
  `1 + size/100 × 15`-pixel cells mixed toward per-pixel jitter by
  `roughness/100`; `color` blends the monochromatic draw toward per-channel
  draws; the seed salts the hash, so equal inputs and seeds are bit-identical.
  The model is a marked approximation of the post-CS6 effect (no Adobe source
  exists for it).

### Slot encoding and defaults

The kind takes a variable slot list `[count, x₀, y₀, blur₀, x₁, y₁, blur₁,
…, bokeh enabled, light bokeh, bokeh color, range low, range high, noise
enabled, amount, size, roughness, color, highlights, kind, seed]`; the arity
table's canonical is one pin plus the 13-slot effects tail
(`FIELD_BLUR_SLOTS = 17`) so `filter_kind_supported`, the gallery stack
resolver, and the generic dialog tests keep working, exactly as Lighting
Effects lists a one-light rig. An empty list maps to one pin at `(0.5, 0.5)`
with blur 0 and the documented effect defaults (Bokeh on 14/40/40..118; Noise
off 0/50/50/0/0/Grain/1); the canonical all-defaults list (count 0) maps to
the same pin with the zeroed effects; `count = 1..=16` requires exactly
`1 + 3·count + 13` slots; any other list is refused. The engine still treats
an empty `pins` vector as the documented zero-pin no-op.

### Workspace dialog

`BlurGalleryDialog` follows the Lighting Effects workspace: a `QDialog` with
its own `BlurGalleryCanvas` (the `LightingCanvas` pattern — the dialog owns
the pin list, the canvas edits it and emits `pinsEdited`/`pinSelected`), a
fixed-width side column, and a proxy preview. The proxy is
`documentPreviewImage(view, 1100)` through `filter_thumbnail("field-blur")`
with each pin's blur scaled by the proxy's document scale, so the preview
matches the commit. Pins are normalized to the document frame; the commit
applies the same fractions to the active layer, so a layer that does not
cover the document previews approximately (the Lighting Effects ceiling).
Dragging inside a pin moves it; dragging its ring changes its blur; a click
on the image adds a pin with the selected pin's blur; Delete removes the
selected pin (never the last one). Every pin is a draggable circle: the
selected one is the larger ring with the blur label, the rest are small
target dots. `H` hides the pin overlay. The Blur Tools panel lists Field
Blur (checked, expanded, editable) and the not-yet-implemented Iris Blur /
Tilt-Shift / Path Blur / Spin Blur rows (disabled). Below it the Effects /
Motion Effects / Noise tabs hold the bokeh and grain controls; the Motion
Effects tab only states that Path/Spin motion effects are unimplemented.
Unchecking Bokeh or Noise greys that tab's controls without losing their
values. Preview / Cancel / OK sit in a bottom bar; OK commits one `"Filter"`
state on a worker, Cancel is bit-exact, and Last Filter / Last Filter
Settings reuse the stored slots like Lens Blur.
The dialog is routed from `runFilterDialog` in `frame_menus_filter.cpp`;
`filter_commands.cpp` carries the canonical one-pin row so the menu is
enabled, labelled `Field Blur…`, and covered by the table-wide tests. The
row sits under a new `Filter ▸ Blur Gallery` submenu — the later Photoshop
structure the owner's screenshot shows; CS6 itself listed the three effects
under `Filter ▸ Blur` — beside disabled `Iris Blur…`, `Tilt-Shift…`,
`Path Blur…`, and `Spin Blur…` stubs (the last two are post-CS6).

## Risks / Trade-offs

- **Approximation.** The field weights and the pyramid are inferred; the
  parity criteria the tests can prove are monotonicity and gradient
  smoothness, not Adobe pixels.
- **Preview mismatch on a partial layer.** Pins normalize to the document
  proxy while the commit normalizes to the layer; acceptable for the
  workspace pattern in use and marked as a ceiling.
- **Memory at commit.** Levels are u8 buffers plus one f64 scratch; at most
  a few document-sized buffers, in line with the existing kernels.

## Open Questions

- CS6's numeric slider range/default and the blur-handle gesture are
  unverified (`FILT-092` open questions); 0..=500 with a 0 default and a
  horizontal ring drag are recorded as inferred.
- The doc's `M` mask view needs a follow-up change; the Noise tab's exact
  modern control set is a secondary source, so its model is inferred.
