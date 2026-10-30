# gpu-compositing Specification

## Purpose
A wgpu compute compositor matching the CPU oracle within one LSB, with graceful CPU fallback and region compositing.

## Requirements

### Requirement: GPU compositor matches the CPU oracle within ±1 LSB

`pictura_render::composite_gpu(doc)` SHALL composite the document's layer stack
on a wgpu compute shader and return the same 4-channel planar straight-alpha
8-bit `PixelBuffer` shape as `pictura_render::composite_rgba(doc)`. For Normal,
every separable blend mode (including the whole-RGB Darken/Lighten comparisons
`DarkerColor` and `LighterColor`), and the four non-separable modes (`Hue`,
`Saturation`, `Color`, `Luminosity`), the GPU output MUST match the CPU oracle
within ±1 LSB per channel.

The GPU path SHALL apply, at the layer's stack position, the five existing
adjustment kinds (invert, posterize, threshold, brightness/contrast,
hue/saturation) and the four lookup-table kinds (levels, curves, exposure,
gradient map), and MUST match the CPU oracle within ±1 LSB. The GPU path MUST
reproduce the CPU semantics for per-layer masks, layer opacity and Fill, pixel
layers, Solid/Gradient/Pattern fill layers, cached raster sources (proxy-less
type, smart-filtered smart object, channel-less smart object), enabled layer
effects, clipping groups, and both pass-through and isolated group compositing.
The CPU compositor remains the oracle and MUST NOT change.

#### Scenario: Separable mode parity

- **WHEN** a document of visible pixel layers using Normal or a separable blend
  mode is composited on a usable Vulkan adapter
- **THEN** every channel of `composite_gpu(doc)` differs from `composite_rgba(doc)` by at most 1 LSB, and the buffer dimensions and channel count are identical

#### Scenario: Non-separable mode parity

- **WHEN** a document of visible pixel layers using `Hue`, `Saturation`, `Color`,
  or `Luminosity` is composited on a usable Vulkan adapter
- **THEN** every channel of `composite_gpu(doc)` differs from `composite_rgba(doc)` by at most 1 LSB

#### Scenario: Adjustment layer parity

- **WHEN** a document contains a visible invert, posterize, threshold,
  brightness/contrast, hue/saturation, levels, curves, exposure, or gradient-map
  adjustment layer
- **THEN** the GPU composite matches `composite_rgba(doc)` within ±1 LSB, with the
  adjustment applied at the same stack position

#### Scenario: Gradients, masks, and opacity

- **WHEN** a layer carries a raster mask, non-255 opacity or Fill, or partial
  alpha, and its blend mode is separable or non-separable
- **THEN** the GPU composite matches the CPU composite within ±1 LSB, including
  in the masked-out and partially transparent regions

#### Scenario: Group semantics

- **WHEN** the stack contains a pass-through group or an isolated group blended
  onto the running canvas
- **THEN** the GPU result matches the CPU result within ±1 LSB for the group and
  its children

#### Scenario: Raster source parity

- **WHEN** the stack contains a proxy-less type layer, a smart-filtered smart
  object, or a channel-less smart-object source
- **THEN** the GPU result matches the CPU result within ±1 LSB, including the
  smart-filter mask mix and the layer mask

#### Scenario: Fill layer parity

- **WHEN** the stack contains a Solid, Gradient, or Pattern fill layer
- **THEN** the GPU result matches the CPU result within ±1 LSB

#### Scenario: Effect and clipping parity

- **WHEN** the stack contains an enabled layer effect or a clipping group
- **THEN** the GPU result matches the CPU result within ±1 LSB

### Requirement: CPU-only modes are rejected before dispatch

`Dissolve` SHALL remain the only CPU-only blend mode: its per-pixel randomness
is not bit-reproducible on the GPU. The four non-separable modes (`Hue`,
`Saturation`, `Color`, `Luminosity`) are supported on the GPU. The nine
supported adjustment kinds (invert, posterize, threshold, brightness/contrast,
hue/saturation, levels, curves, exposure, gradient map) are applied on the GPU;
any other adjustment kind SHALL make `composite_gpu` return
`GpuError::UnsupportedAdjustment` for that layer without dispatching it to the
shader and without panicking. A document with a non-default Blend If range, a
knockout, or a recorded source depth SHALL decline before any dispatch with the
matching error. Every decline SHALL name one reason and MUST NOT panic.

#### Scenario: Dissolve mode

- **WHEN** `composite_gpu` is called on a document whose visible layer uses
  `Dissolve`
- **THEN** it returns `Err(GpuError::UnsupportedMode(BlendMode::Dissolve))`

#### Scenario: Supported adjustment composites

- **WHEN** `composite_gpu` is called on a document whose visible adjustment layer
  is one of the nine supported kinds
- **THEN** it returns `Ok` and applies the adjustment at the layer's stack
  position rather than erroring

#### Scenario: Unsupported adjustment kind

- **WHEN** `composite_gpu` is called on a document whose visible layer is an
  adjustment layer of any other kind
- **THEN** it returns `Err(GpuError::UnsupportedAdjustment)`

#### Scenario: Advanced blending still declines

- **WHEN** `composite_gpu` is called on a document whose visible layer carries a
  non-default Blend If range or a knockout
- **THEN** it returns `Err(GpuError::UnsupportedAdvancedBlending)`

### Requirement: Graceful CPU fallback when no GPU is available

The GPU path MUST NOT panic when no usable adapter exists. Adapter/device
creation failure, documents exceeding device buffer or workgroup limits, and
readback failure SHALL each produce a `GpuError` variant
(`Unavailable`, `TooLarge`, `Readback`). `composite_gpu_or_cpu(doc)` MUST return
the CPU compositor's result whenever `composite_gpu` returns any `GpuError`, and
its output for a CPU-only stack MUST be byte-identical to
`composite_rgba(doc)`.

#### Scenario: No Vulkan adapter

- **WHEN** `composite_gpu` runs on a machine with no usable Vulkan adapter
- **THEN** it returns `Err(GpuError::Unavailable)` and does not panic, and
  `composite_gpu_or_cpu` returns the CPU composite

#### Scenario: Document exceeds device limits

- **WHEN** the document's buffers or dispatch count exceed the device limits
- **THEN** `composite_gpu` returns `Err(GpuError::TooLarge)` and does not panic

#### Scenario: Fallback for a CPU-only stack

- **WHEN** a document contains a color-luminosity or Dissolve layer and is passed
  to `composite_gpu_or_cpu`
- **THEN** the returned buffer equals `composite_rgba(doc)` byte for byte

### Requirement: GPU-native compositing data path

The GPU compositor SHALL consume each layer's pixel data as 8-bit planar channel
samples over the layer's clamped rect, uploaded directly, without a host-side
full-canvas floating-point assembly of the source or mask. The compute shader
SHALL sample the source channels and the layer-mask coverage itself, the working
canvas SHALL be kept as packed 8-bit RGBA rather than `f32`, and the compositor
SHALL return packed 8-bit output de-interleaved to the planar `PixelBuffer`.
Output MUST remain identical in shape to `composite_rgba` and within ±1 LSB per
channel of the CPU oracle. The public API (`composite_gpu`, `composite_active`,
`gpu_available`, `Backend`, `GpuError`) SHALL be unchanged.

#### Scenario: Parity after the data-path change

- **WHEN** a document using Normal, a separable or non-separable blend mode, or
  one of the supported adjustment layers is composited through the 8-bit planar
  data path on a usable adapter
- **THEN** every channel of the GPU result differs from `composite_rgba(doc)` by
  at most 1 LSB, and the CPU oracle is unchanged

#### Scenario: Output shape is unchanged

- **WHEN** any document is composited through the GPU-native data path
- **THEN** the returned `PixelBuffer` has the same dimensions and channel count
  as `composite_rgba(doc)`

#### Scenario: Group layer dispatch

- **WHEN** the stack contains a pass-through or isolated group
- **THEN** the group dispatch binds the group's inner packed 8-bit canvas as its
  source without re-materializing it to `f32`, and the GPU result still matches
  the CPU oracle within ±1 LSB

### Requirement: Large-document GPU dispatch

The GPU compositor SHALL dispatch its compute shader as a two-dimensional grid
(an equivalent tiling) so that any document whose pixel count exceeds the
one-dimensional workgroup limit (`max_compute_workgroups_per_dimension`, 65535
workgroups × 64 threads ≈ 4.19 MP) still runs on the GPU rather than falling back
to the CPU. The shader SHALL derive the linear pixel index from a row stride
passed in the uniform, so that no invocation is skipped or double-covered,
preserving the existing ±1 LSB parity with the CPU oracle and the existing
`GpuError` semantics. A document whose pixel count exceeds the two-dimensional
workgroup product (`limit × limit × 64`) SHALL return `GpuError::TooLarge`
without panicking.

#### Scenario: A document above the one-dimensional limit composites on the GPU

- **WHEN** a document whose pixel count exceeds ~4.19 MP (for example 4000×4000)
  is composited with GPU compute enabled on a machine with a usable adapter
- **THEN** the reported backend is `Backend::Gpu`, every channel differs from
  `composite_rgba(doc)` by at most 1 LSB, and the alpha channel is unchanged

#### Scenario: A document above the two-dimensional product limit is rejected

- **WHEN** `composite_gpu` is called on a document whose pixel count exceeds
  `max_compute_workgroups_per_dimension × max_compute_workgroups_per_dimension ×
  64`
- **THEN** it returns `Err(GpuError::TooLarge)` and does not panic

### Requirement: Region compositing

`pictura_render::composite_region_active(doc, rect, gpu_enabled)` SHALL composite
only the requested document rectangle through the active backend and return a
rectangle-sized buffer. Its full signature is
`(doc: &Document, rect: PsdRect, gpu_enabled: bool) -> (PixelBuffer, Backend)`.
The `rect` SHALL be clamped to the document bounds, and the returned
`PixelBuffer` SHALL be sized `rect.width() × rect.height()` over the clamped
rectangle. For separable modes, non-separable modes, adjustment layers, groups,
masked layers, fill layers, cached raster sources, enabled layer effects, and
clipping groups, the returned buffer MUST be byte-identical to the corresponding
sub-rectangle of `composite_active(doc, gpu_enabled)`. The full-document
`composite_active` SHALL remain the oracle and MUST NOT change. A `rect` whose
intersection with the document is empty SHALL return a zero-dimension buffer and
MUST NOT panic.

#### Scenario: A region matching the full canvas equals the full composite

- **WHEN** `composite_region_active` is called with a rect equal to the whole
  document rect
- **THEN** the returned buffer is byte-identical to
  `composite_active(doc, gpu_enabled)` and reports the same `Backend`

#### Scenario: A sub-rect equals the corresponding slice of the full composite

- **WHEN** a document using a separable mode, a non-separable mode, an adjustment
  layer, a group, a masked layer, a fill layer, a cached raster source, an
  effect, or a clipping group is composited over a sub-rectangle smaller than the
  canvas
- **THEN** every pixel of the returned buffer equals the same pixel of
  `composite_active(doc, gpu_enabled)` at the rect offset

#### Scenario: Out-of-bounds and empty regions are clamped without panicking

- **WHEN** `composite_region_active` is called with a rect that extends outside
  the document, or that has zero or negative area
- **THEN** it clamps the rect to the document intersection and returns a buffer
  of the intersection's size, or a zero-dimension buffer when the intersection is
  empty, and does not panic

### Requirement: Row-wise planar source assembly

`Gpu::build_source` SHALL assemble a pixel layer's planar 8-bit source planes
with whole-row copies (`copy_from_slice`/`clone_from_slice`) when each channel
plane covers the layer-rect ∩ region row intersection, instead of a per-pixel
loop. It MUST preserve the existing semantics exactly: planar straight-alpha
planes; a grayscale document produces two planes with colour read from channel 0;
an absent green or blue channel aliases channel 0; an absent alpha plane reads
255; an absent colour channel reads 0. When a channel plane does not cover the
intersection (a short or absent plane, or channel 0 absent) the assembly SHALL
fall back to the existing per-pixel path. The assembled plane bytes MUST be
byte-identical (0 LSB) to the per-pixel assembly.

#### Scenario: A covering plane assembles row-wise and is byte-identical

- **WHEN** `build_source` assembles a layer whose channel data covers the whole
  clamped row intersection
- **THEN** the planes are filled with whole-row copies and are byte-identical to
  the per-pixel assembly of the same layer

#### Scenario: A short or absent plane falls back without changing bytes

- **WHEN** a channel plane is short over the clamped row intersection, or the
  green/blue channel is absent (aliasing channel 0), or the alpha plane is absent
- **THEN** the assembly falls back to the per-pixel path, the absent alpha plane
  reads 255, and the planes remain byte-identical to the per-pixel assembly

### Requirement: Row-wise mask coverage assembly

`Gpu::build_mask` SHALL fill the coverage plane with row-wise operations when the
layer has no enabled data-carrying mask (`mask` is absent, `disabled`, or carries
no data): coverage 255 inside the layer/group/adjustment influence rectangle and 0
outside. When the mask is enabled and carries data it SHALL keep the per-pixel
`mask_alpha` path. The coverage plane MUST be byte-identical (0 LSB) to the
per-pixel coverage.

#### Scenario: A maskless layer is filled row-wise and is byte-identical

- **WHEN** `build_mask` runs for a layer with no enabled data-carrying mask
- **THEN** the influence rectangle is filled 255 and the remainder stays 0, and
  the plane is byte-identical to the per-pixel `mask_alpha` coverage

#### Scenario: A data-carrying mask keeps the per-pixel path

- **WHEN** `build_mask` runs for a layer whose enabled mask carries pixel data
- **THEN** the per-pixel `mask_alpha` path is used and the plane is byte-identical
  to the previous coverage

### Requirement: Fused planar readback

The canvas readback SHALL de-interleave the mapped staging bytes directly into
the planar straight-alpha `PixelBuffer` and MUST NOT allocate or copy a host-side
packed RGBA intermediate. The returned `PixelBuffer` MUST be byte-identical
(0 LSB) to the previous packed readback followed by the planar de-interleave.

#### Scenario: The readback equals the packed-then-de-interleaved output

- **WHEN** a composite is read back through the fused path
- **THEN** the returned `PixelBuffer` is byte-identical to the output of the
  previous packed readback and `to_pixel_buffer` de-interleave

### Requirement: GPU-side canvas initialization

The working canvas SHALL be initialized on the GPU with a command-buffer clear
over a storage buffer that carries `COPY_DST` usage, rather than by allocating and
uploading a full-canvas host zero buffer. The clear SHALL be submitted before the
first composite dispatch that reads that canvas, so no dispatch observes
uninitialized canvas memory and the canvas bytes MUST be identical to the
previously uploaded zero canvas.

#### Scenario: A GPU-cleared canvas composites identically

- **WHEN** a composite runs on a canvas initialized by the command-buffer clear,
  including a group whose inner canvas is also cleared on the GPU
- **THEN** the composited output is byte-identical to the same composite on an
  uploaded zero canvas

### Requirement: Full-composite throughput evidence

`composite_gpu` MUST remain byte-identical (0 LSB) to itself across repeated
composites and to the region path, and within ±1 LSB per channel of
`composite_rgba(doc)` with the alpha channel unchanged. An `#[ignore]` profile
test SHALL print the full-composite phase timings (canvas clear, source assembly,
mask assembly, readback de-interleave, and the total) for a 4000×4000
two-pixel-layer document so the improvement is measurable, and SHALL skip with a
printed note when no usable adapter exists. The profile test SHALL NOT assert a
throughput ratio.

#### Scenario: The parity contract holds after the data-path change

- **WHEN** the `gpu_parity` scenes are composited through the row-wise assembly,
  fused readback, and GPU-side clear on a usable adapter
- **THEN** repeated GPU composites of the same document are byte-identical, the
  region composite equals the full-composite slice at 0 LSB, every channel
  differs from `composite_rgba(doc)` by at most 1 LSB, and the alpha channel is
  unchanged

#### Scenario: The ignored profile test prints the phase timings

- **WHEN** the `#[ignore]` profile test is run with `--ignored --nocapture` on a
  machine with a usable adapter
- **THEN** it prints the per-phase timings and the total for the 4000×4000
  two-pixel-layer composite

#### Scenario: A full composite stays within a small time budget

- **WHEN** a 4000×4000 two-pixel-layer document is composited through the full
  GPU path after the warm-up composite
- **THEN** the composite completes within a small time budget

### Requirement: Row-parallel CPU fallback composite

The CPU compositor SHALL evaluate independent output rows in parallel. Its output
MUST be byte-identical to the single-threaded composite for every layer stack, and
the CPU oracle's output MUST NOT change: `composite_rgba` returns exactly the bytes
it returned before this change. Only its execution is parallelised. Layer order,
group and knockout topology, and neighborhood effect kernels (blur-, offset-, and
shape-based layer effects) SHALL remain sequential; only per-pixel content loops
whose rows are independent MAY run in parallel. The GPU compositor remains the
preferred backend; this parallelism accelerates the CPU fallback when no adapter is
usable, the stack is unsupported on the GPU, or the document exceeds the device
buffer limits.

#### Scenario: Parallel output equals the sequential oracle

- **WHEN** a document is composited through the parallel CPU path and through the
  single-threaded oracle
- **THEN** the two buffers are byte-identical

#### Scenario: The CPU oracle's output is unchanged

- **WHEN** `composite_rgba` is run before and after the parallelism change on the
  same document
- **THEN** it returns byte-identical output; only its execution differs

#### Scenario: A stack with an effect or knockout is unaffected

- **WHEN** a document containing a layer effect or a knockout layer is composited
- **THEN** the result is byte-identical to the single-threaded composite of the
  same document

### Requirement: The GPU compositor is validated against the CPU oracle before use

`gpu_available()` SHALL report a GPU as available only when a usable Vulkan
device exists AND that device reproduces the CPU oracle `composite_rgba` on a
representative scene within ±1 LSB per channel. The representative scene SHALL
include partial alpha and a separable blend, so a device that composites only a
few pixels or drops alpha is detected. The check SHALL run at most once per
process and its result SHALL be cached. When the check fails, `composite_gpu`
SHALL return `GpuError::Unavailable` for a supported document, `composite_active`
and `composite_gpu_or_cpu` SHALL keep the CPU path, `shared_device()` SHALL
return `None` so the GPU filter kernels also fall back to the CPU, and the GPU
paint stroke SHALL fall back to the CPU: `GpuStroke::available()` SHALL report
false and `GpuStroke::new` SHALL return `GpuError::Unavailable`, so a
miscompiled `STROKE_SHADER` never commits wrong paint pixels. The eligibility
check SHALL still precede the availability gate, so a `Dissolve` stack SHALL
report `GpuError::UnsupportedMode` even when the device is unusable. The public
signatures of `composite_gpu`, `composite_active`, `gpu_available`, and
`Backend` SHALL be unchanged. `GpuError` MAY gain variants that name a reason
for declining; its existing variants SHALL be unchanged.

#### Scenario: A device that matches the oracle stays enabled

- **WHEN** a usable Vulkan device composites the representative scene and every
  channel differs from `composite_rgba` by at most 1 LSB
- **THEN** `gpu_available()` returns true, `composite_gpu` returns `Ok`, and
  `composite_active(doc, true)` reports `Backend::Gpu`

#### Scenario: A device that diverges is treated as absent

- **WHEN** a usable Vulkan device composites the representative scene and any
  channel differs from `composite_rgba` by more than 1 LSB
- **THEN** `gpu_available()` returns false, `composite_gpu` returns
  `Err(GpuError::Unavailable)`, `composite_active(doc, true)` reports
  `Backend::Cpu` with bytes equal to `composite_rgba(doc)`, and `shared_device()`
  returns `None`

#### Scenario: The GPU paint stroke falls back on a diverging device

- **WHEN** the validation fails and the app tries to seed a GPU paint stroke
- **THEN** `GpuStroke::available()` returns false and `GpuStroke::new` returns
  `Err(GpuError::Unavailable)`, so the CPU stroke is used and no GPU dab patches
  the document

#### Scenario: The CPU-only mode check still precedes the gate

- **WHEN** `composite_gpu` is called on a document whose visible layer uses
  `Dissolve` on a device that fails the validation
- **THEN** it returns `Err(GpuError::UnsupportedMode(BlendMode::Dissolve))`, not
  `Unavailable`

#### Scenario: The check runs at most once

- **WHEN** `gpu_available()` is called repeatedly on the same process
- **THEN** the representative scene is composited at most once and the cached
  result is returned

### Requirement: High-bit-depth documents composite on the CPU

The CPU compositor reads native-precision samples whenever a document records a
source depth. That covers a 16- or 32-bit read and a mode converted from a
non-8-bit source. For any such document, `composite_gpu` SHALL return
`GpuError::UnsupportedDepth`. `composite_active` and the region path SHALL
return the CPU result with `Backend::Cpu`. A document with no source depth SHALL
remain GPU-eligible.

#### Scenario: A 16-bit document declines

- **WHEN** the eligibility check runs on a document whose source depth is 16 bits
- **THEN** it returns `Err(GpuError::UnsupportedDepth)`

#### Scenario: The active backend falls back for a 32-bit document

- **WHEN** `composite_active(doc, true)` is called on a 32-bit document
- **THEN** it reports `Backend::Cpu` and returns bytes equal to
  `composite_rgba(doc)`

#### Scenario: An 8-bit document stays eligible

- **WHEN** the eligibility check runs on a document with no source depth and only
  supported layers
- **THEN** it accepts the document

### Requirement: One plan per layer drives eligibility and dispatch

The compositor SHALL compute one support plan per visible layer and SHALL use
that same plan both to decide document eligibility and to choose the layer's
dispatch. Eligibility SHALL be the conjunction of the plans: the document runs on
the GPU when every visible layer is accepted, and the first declining plan in
stack order names the decline reason. The dispatcher MUST NOT re-derive the
route, and a layer whose plan declined MUST NOT be dispatched. A hidden layer
SHALL NOT be dispatched. A hidden clipping base SHALL suppress its clipped run,
so the run is neither planned nor dispatched. Computing the plan MUST NOT read
the GPU device and MUST NOT change any pixel.

#### Scenario: Plan and dispatch agree for every content route

- **WHEN** a one-layer document is built for each content route, both accepted
  and declined
- **THEN** an accepted route dispatches exactly the source its plan names, and a
  declined route returns its error without a dispatch

#### Scenario: A hidden layer is not dispatched

- **WHEN** a document contains a hidden layer carrying an enabled effect or an
  unsupported adjustment
- **THEN** the hidden layer does not decline the document and contributes no
  dispatch

#### Scenario: A hidden clipping base suppresses its run

- **WHEN** a visible clipped layer sits above a hidden clipping base
- **THEN** the clipped run contributes no dispatch and the result matches the
  CPU, which also hides the run

#### Scenario: The plan is device-independent

- **WHEN** the plan is computed on a machine with no usable Vulkan adapter
- **THEN** it produces the same accept/decline result as on a machine with one

### Requirement: GPU eligibility accepts the common content routes

The compositor SHALL classify each visible layer's content into exactly one
content route:

- isolated group;
- pass-through group;
- decoded fill or adjustment;
- undecodable adjustment (a CPU no-op);
- undecodable vector fill;
- smart-filtered smart-object source;
- channel-less smart-object source;
- channel-less type source;
- pixels.

The CPU compositor SHALL select its content path from this classification. The
GPU plan SHALL accept the pixel route, the three cached raster routes, the
decoded fill route (including a decoded `vscg` vector fill), the decoded
adjustment route, enabled effects, and clipping. The smart-filtered route
requires at least one individually enabled filter and every enabled filter to
decode; the filter group's master switch does not change the route. A cached
raster route whose buffer cannot render SHALL fall through to the pixel route
when the layer carries a proxy; with no proxy it SHALL decline with
`UnsupportedSmartObject` (smart object) or `UnsupportedText` (type). The
undecodable adjustment and undecodable vector fill routes SHALL decline with
`UnsupportedAdjustment`. The `Dissolve` mode and non-default Blend If and
knockout SHALL decline with their existing errors. A document whose source depth
is set SHALL decline before any layer is planned.

The first layer in stack order that declines SHALL determine the reason, and
within a layer the checks SHALL run in this order: content route, `Dissolve`,
then advanced blending. Every decline SHALL return a `GpuError` before any
dispatch and MUST NOT panic. The classification MUST NOT change the CPU output:
`composite_rgba` SHALL return exactly the bytes it returned before this change.

Blend Text Colors Using Gamma SHALL NOT be an eligibility attribute for a type
layer that carries a raster proxy: such a layer SHALL take the pixel route and
the blend shader SHALL apply the curve. A channel-less type source SHALL be
accepted through its cached raster; a channel-less type layer whose raster cannot
render SHALL decline with `UnsupportedText`. The GPU decline counter SHALL keep
the existing reason names.

#### Scenario: Every declining content route has a reason

- **WHEN** the eligibility check runs on a one-layer document for each content
  route the GPU does not implement
- **THEN** it returns that route's `GpuError`, with no GPU device needed

#### Scenario: A proxied type layer stays eligible with the gamma on

- **WHEN** the eligibility check runs on a document whose only layer is a type
  layer carrying a raster proxy, with Blend Text Colors Using Gamma on
- **THEN** it accepts the document

#### Scenario: A channel-less type layer is accepted

- **WHEN** `composite_gpu` runs on a document whose only layer is a type layer
  with no raster proxy and renderable text
- **THEN** it accepts the document and composites the cached type raster

#### Scenario: An unrenderable channel-less type layer declines

- **WHEN** the eligibility check runs on a channel-less type layer whose raster
  cannot render (for example an empty text model)
- **THEN** it returns `Err(GpuError::UnsupportedText)` and counts `type_source`

#### Scenario: An unrenderable proxy-less smart object declines

- **WHEN** the eligibility check runs on a channel-less smart object whose
  embedded source cannot render
- **THEN** it returns `Err(GpuError::UnsupportedSmartObject)`

#### Scenario: A decoded vector fill is accepted

- **WHEN** a layer carries a decodable `vscg` fill and no adjustment block
- **THEN** the eligibility check accepts it and it takes the fill route

#### Scenario: Effects and clipping no longer decline

- **WHEN** the eligibility check runs on a document whose only layer carries an
  enabled drop shadow, or whose stack contains a clipping group
- **THEN** it accepts the document

#### Scenario: The CPU oracle is unchanged

- **WHEN** one document per content route is composited by `composite_rgba`
- **THEN** each result equals the bytes recorded before the change

### Requirement: Layer effects composite on the GPU

Enabled layer-effect passes SHALL be replayed as GPU dispatches in the same order
as the CPU compositor: below-content passes (drop shadow, outer glow) before the
layer's content, above-content passes (inner shadow and any effect the CPU draws
above) after it. Each pass SHALL keep its document-space rectangle, blend mode,
constant or per-pixel colour, per-pixel alpha, and the layer's effect opacity
scaling. The dispatch SHALL use constant coverage 255 and `fill = 1` because the
pass alpha already includes the layer mask and the effect's own opacity, and it
SHALL NOT apply the text-gamma curve. A disabled, absent, or undecodable effect,
and an enabled effect the CPU resolves to a no-op (a bevel in a style the CPU
does not render, a legacy effect that maps to no renderer, or a stroke or overlay
whose payload cannot decode), SHALL contribute no pass and MUST NOT decline the
document. The GPU output MUST match the CPU oracle within ±1 LSB per channel.

#### Scenario: A drop-shadow layer composites on the GPU

- **WHEN** `composite_gpu` runs on a document whose visible layer carries an
  enabled and present drop shadow
- **THEN** it returns `Ok`, the shadow appears at the CPU's position, and the
  result matches `composite_rgba` within ±1 LSB

#### Scenario: Every effect family matches the CPU

- **WHEN** a document contains a layer with an enabled outer glow, inner glow,
  inner shadow, bevel and emboss, satin, solid/gradient/pattern stroke, or
  color/gradient/pattern overlay
- **THEN** the GPU composite matches `composite_rgba` within ±1 LSB for that
  family

#### Scenario: A multi-effect layer keeps its draw order

- **WHEN** a layer enables a drop shadow, a stroke, and an inner shadow
- **THEN** the passes dispatch in the CPU's below/content/above order and the
  result matches the CPU within ±1 LSB

#### Scenario: A disabled or undecodable effect stays eligible

- **WHEN** a layer's only effect is disabled, or its effect block cannot decode
- **THEN** the layer is accepted, no pass is dispatched for it, and the result
  matches the CPU

#### Scenario: An enabled effect the CPU renders as a no-op stays eligible

- **WHEN** a layer carries a Stroke-style bevel, a legacy effect that maps to no
  renderer, or a stroke/overlay whose payload cannot decode
- **THEN** the layer is accepted, no pass is dispatched for it, and the result
  matches the CPU

#### Scenario: Effect region parity

- **WHEN** a styled document is composited over a sub-region and whole
- **THEN** every pixel of the region composite equals the same pixel of the full
  composite

### Requirement: Clipping groups composite on the GPU

A clipping group SHALL be composited through the GPU backend using the CPU's
sibling-group algorithm. The group buffer SHALL be built with that algorithm over
the requested region, uploaded as a packed source, and blended onto the GPU
canvas; the base SHALL be composited at full opacity, Normal mode, into an empty
group canvas; the group coverage SHALL be the base's alpha composited alone at
full opacity and Fill without effects; each clipped layer SHALL be composited
onto a copy of the group and the group SHALL be moved toward that copy by the
coverage; and the group SHALL then be blended onto the running canvas with the
base's opacity and blend mode, `fill = 1`, and no layer mask or vector mask (the
base's mask was already applied inside the group). A hidden base SHALL hide its
clipped run; a clipped layer with no base SHALL composite unclipped; the base's
Fill and effects MUST NOT narrow the coverage. The GPU output MUST match the CPU
oracle within ±1 LSB.

#### Scenario: A clipping group matches the CPU

- **WHEN** a document has a green layer clipped to a red base over a backdrop
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB, with green only
  over the base's content

#### Scenario: A hidden base hides the run

- **WHEN** the clipping base is hidden
- **THEN** the clipped layers contribute nothing on the GPU, matching the CPU

#### Scenario: A clipped layer with no base is unclipped

- **WHEN** the bottom layer of a stack is clipped
- **THEN** it composites unclipped on the GPU, matching the CPU

#### Scenario: Multiple clipped layers share one base

- **WHEN** two clipped layers sit above one base
- **THEN** both are clipped by the same coverage and the GPU result matches the
  CPU within ±1 LSB

#### Scenario: The base's Fill and effects do not narrow the clip

- **WHEN** the base has Fill below 255 or an enabled effect
- **THEN** the coverage is still the base's content alpha and the GPU result
  matches the CPU

#### Scenario: Clipping region parity

- **WHEN** a clipping group is composited over a sub-region and whole
- **THEN** every pixel of the region composite equals the same pixel of the full
  composite

### Requirement: Fill layers rasterize on the CPU and upload as pixels

A Solid, Gradient, or Pattern fill layer SHALL be rasterized over its layer rect
on the CPU and uploaded as an ordinary pixel source, so the GPU applies the same
mask, opacity, Fill, and blend mode as the CPU. A decoded `vscg` vector fill SHALL
take the same route. The rasterized buffer MUST be the exact straight-alpha RGBA
the CPU would blend. A Pattern fill reads the document pattern library; its upload
SHALL NOT be retained across composites unless the retention key covers the
pattern content.

#### Scenario: A solid fill matches the CPU

- **WHEN** a document contains a visible Solid Color fill layer
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

#### Scenario: A gradient fill matches the CPU

- **WHEN** a document contains a visible Gradient fill layer
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB for every
  gradient kind and angle

#### Scenario: A pattern fill matches the CPU

- **WHEN** a document contains a visible Pattern fill layer
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

#### Scenario: A pattern fill is not served stale

- **WHEN** the document's pattern content changes and the same fill layer is
  composited again
- **THEN** the new pattern is rendered, matching a purge-then-composite

#### Scenario: A decoded vector fill matches the CPU

- **WHEN** a layer carries a decodable `vscg` Solid, Gradient, or Pattern fill
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

### Requirement: Adjustment lookup tables match the CPU

Levels, Curves, Exposure, and Gradient Map SHALL be applied through a 256-entry
lookup table built by the same adjustment code the CPU applies, so the GPU does
not re-derive the math. Levels, Curves, and Exposure SHALL use one combined
per-channel table (the channel table composed with the composite table where the
CPU applies both). Gradient Map SHALL use a luma-indexed table carrying the
mapped colour and its alpha; the index SHALL use the CPU's Rec.601 weights
(`0.299 R + 0.587 G + 0.114 B`) and the same rounding. The GPU output MUST match
the CPU oracle within ±1 LSB per channel.

#### Scenario: Levels matches the CPU

- **WHEN** a document contains a visible Levels adjustment layer with composite
  and per-channel records
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

#### Scenario: Curves matches the CPU

- **WHEN** a document contains a visible Curves adjustment layer with a
  composite curve and per-channel curves
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

#### Scenario: Exposure matches the CPU

- **WHEN** a document contains a visible Exposure adjustment layer
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB

#### Scenario: Gradient Map matches the CPU

- **WHEN** a document contains a visible Gradient Map adjustment layer with
  partially transparent stops
- **THEN** the GPU result matches `composite_rgba` within ±1 LSB, including the
  blend with each pixel's original colour

#### Scenario: A masked LUT adjustment is gated like the CPU

- **WHEN** a Levels, Curves, Exposure, or Gradient Map layer carries an enabled
  raster mask and reduced opacity
- **THEN** the GPU result matches the CPU within ±1 LSB
