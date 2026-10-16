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
within ±1 LSB per channel. The GPU path SHALL likewise apply the five supported
adjustment layers (invert, posterize, threshold, brightness/contrast,
hue/saturation) at the layer's stack position and MUST match the CPU oracle
within ±1 LSB. The GPU path MUST reproduce the CPU semantics for per-layer masks,
layer opacity, clipping-free pixel layers, and both pass-through and isolated
group compositing. The CPU compositor remains the oracle and MUST NOT change.

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
  brightness/contrast, or hue/saturation adjustment layer
- **THEN** the GPU composite matches `composite_rgba(doc)` within ±1 LSB, with the
  adjustment applied at the same stack position

#### Scenario: Gradients, masks, and opacity

- **WHEN** a layer carries a raster mask, non-255 opacity, or partial alpha, and
  its blend mode is separable or non-separable
- **THEN** the GPU composite matches the CPU composite within ±1 LSB, including
  in the masked-out and partially transparent regions

#### Scenario: Group semantics

- **WHEN** the stack contains a pass-through group or an isolated group blended
  onto the running canvas
- **THEN** the GPU result matches the CPU result within ±1 LSB for the group and
  its children

### Requirement: CPU-only modes are rejected before dispatch

`Dissolve` SHALL be the only CPU-only blend mode: its per-pixel randomness is
not bit-reproducible on the GPU. The four non-separable modes (`Hue`,
`Saturation`, `Color`, `Luminosity`) are supported on the GPU. The five
supported adjustment kinds (invert, posterize, threshold, brightness/contrast,
hue/saturation) are applied on the GPU; any other adjustment kind SHALL make
`composite_gpu` return `GpuError::UnsupportedAdjustment` for that layer without
dispatching it to the shader and without panicking.

#### Scenario: Dissolve mode

- **WHEN** `composite_gpu` is called on a document whose visible layer uses
  `Dissolve`
- **THEN** it returns `Err(GpuError::UnsupportedMode(BlendMode::Dissolve))`

#### Scenario: Supported adjustment composites

- **WHEN** `composite_gpu` is called on a document whose visible adjustment layer
  is one of the five supported kinds
- **THEN** it returns `Ok` and applies the adjustment at the layer's stack
  position rather than erroring

#### Scenario: Unsupported adjustment kind

- **WHEN** `composite_gpu` is called on a document whose visible layer is an
  adjustment layer of any other kind
- **THEN** it returns `Err(GpuError::UnsupportedAdjustment)`

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
and masked layers, the returned buffer MUST be byte-identical to the
corresponding sub-rectangle of
`composite_active(doc, gpu_enabled)`. The full-document `composite_active` SHALL
remain the oracle and MUST NOT change. A `rect` whose intersection with the
document is empty SHALL return a zero-dimension buffer and MUST NOT panic.

#### Scenario: A region matching the full canvas equals the full composite

- **WHEN** `composite_region_active` is called with a rect equal to the whole
  document rect
- **THEN** the returned buffer is byte-identical to
  `composite_active(doc, gpu_enabled)` and reports the same `Backend`

#### Scenario: A sub-rect equals the corresponding slice of the full composite

- **WHEN** a document using a separable mode, a non-separable mode, an adjustment
  layer, a group, or a masked layer is composited over a sub-rectangle smaller
  than the canvas
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

### Requirement: Layer effects are rejected before GPU dispatch

A visible layer carrying a decodable layer effect SHALL make `composite_gpu`
return `GpuError::UnsupportedLayerEffect` before dispatching that layer to the
GPU, without panicking. A layer counts as effect-bearing when the effects the
compositor resolves for it decode to an enabled and present `DropShadow`, an
enabled and present `OuterGlow`, an enabled and present `InnerShadow`, an
enabled and present `InnerGlow`, an enabled and present `BevelEmboss`, an enabled
and present `Satin`, an enabled and present `Stroke` (solid, gradient, or
pattern fill), an enabled and present `ColorOverlay`, an enabled and present
`GradientOverlay`, or an enabled and present `PatternOverlay`. The effects SHALL
be resolved through the same shared decode path the CPU compositor uses: an
`lfx2` block, or, when `lfx2` is absent, the layer's legacy `lrFX`
(`EFFECTS_LAYER`) block mapped into the typed effect set, with `lfx2` taking
precedence when both are present. A disabled, absent, or malformed effect, a
bevel whose style or technique is not rendered, a stroke whose gradient or
pattern fill payload cannot be decoded, a legacy effect that maps to no renderer,
an overlay whose pattern or gradient payload cannot be decoded, and a legacy
block that does not decode SHALL NOT reject the document. `composite_active` and
`composite_gpu_or_cpu` SHALL fall back to the CPU composite for a document with
such a layer, and the fallback output SHALL be byte-identical to `composite_rgba`
of the same document.

#### Scenario: A drop-shadow layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present drop shadow
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: An outer-glow layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present outer glow
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: An inner-shadow layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present inner shadow
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: An inner-glow layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present inner glow
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: A bevel and emboss layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present bevel and emboss
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: A satin layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present satin
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: A solid stroke layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present solid-colour stroke
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: A gradient or pattern stroke layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present gradient-fill stroke, or an enabled and present pattern-fill stroke
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: An overlay layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries an enabled and present color overlay, gradient overlay, or pattern overlay
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: A legacy effects layer is rejected before dispatch

- **WHEN** `composite_gpu` is called on a document whose visible layer carries only a legacy `lrFX` block decoding to an enabled and present drop shadow, outer glow, inner shadow, inner glow, bevel, or solid fill
- **THEN** it returns `Err(GpuError::UnsupportedLayerEffect)` and does not panic

#### Scenario: The effect document falls back to the CPU composite

- **WHEN** `composite_gpu_or_cpu` is called on a document whose visible layer carries an enabled and present object-based or legacy effect
- **THEN** it returns the same buffer as `composite_rgba` for that document

#### Scenario: A disabled or undecodable effect does not reject the GPU

- **WHEN** `composite_gpu` is called on a document whose only effect is disabled, whose stroke gradient/pattern payload cannot be decoded, whose overlay pattern/gradient payload cannot be decoded, whose legacy block is malformed or maps to a bevel style that is not rendered, or whose legacy block resolves to no effect
- **THEN** the effect does not by itself produce `UnsupportedLayerEffect`

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
miscompiled `STROKE_SHADER` never commits wrong paint pixels. The CPU-only
blend-mode check SHALL still precede the availability gate, so a `Dissolve` stack
SHALL report `GpuError::UnsupportedMode` even when the device is unusable. The
public signatures of `composite_gpu`, `composite_active`, `gpu_available`,
`Backend`, and `GpuError` SHALL be unchanged.

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
