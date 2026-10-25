# Spec Delta

## ADDED Requirements

### Requirement: GPU eligibility follows the CPU content route

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

The CPU compositor SHALL select its content path from this classification.
`composite_gpu` SHALL decide eligibility per content route. It SHALL also check
these layer attributes separately:

- enabled object-based effects;
- the `Dissolve` mode;
- non-default Blend If, knockout, and clipping;
- Blend Text Colors Using Gamma on a type layer.

For a document whose source depth is set, `composite_gpu` SHALL decline before
looking at any layer. Otherwise the first layer in stack order that declines
SHALL determine the reason, and within a layer the checks SHALL run in this
order: effects, content route, `Dissolve`, then advanced blending. Every decline
SHALL return a `GpuError` before any dispatch and MUST NOT panic. The
classification MUST NOT change the CPU output: `composite_rgba` SHALL return
exactly the bytes it returned before this change.

#### Scenario: Every declining content route has a reason

- **WHEN** the eligibility check runs on a one-layer document for each content
  route the GPU does not implement
- **THEN** it returns that route's `GpuError`, with no GPU device needed

#### Scenario: Effects win over the content route

- **WHEN** a fill layer carries an enabled drop shadow
- **THEN** the eligibility check returns `GpuError::UnsupportedLayerEffect`

#### Scenario: The CPU oracle is unchanged

- **WHEN** one document per content route is composited by `composite_rgba`
- **THEN** each result equals the bytes recorded before the change

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

### Requirement: Smart-filtered layers composite on the CPU

A visible smart-object layer whose individually enabled smart filters all
decode, with at least one of them enabled, takes the CPU's smart-filtered route.
The CPU re-renders the embedded source for that route, so its pixel proxy is
ignored. This holds whether the filter group's master switch is on or off.
`composite_gpu` SHALL return `GpuError::UnsupportedSmartFilter` for such a
layer. A smart object with no individually enabled filter, or with an enabled
filter the engine cannot decode, does not take that route. Such a layer SHALL
remain GPU-eligible when its proxy pixels are present.

#### Scenario: An enabled Camera Raw filter declines

- **WHEN** the eligibility check runs on a document with a pixel-carrying smart
  object whose Camera Raw smart filter is enabled
- **THEN** it returns `Err(GpuError::UnsupportedSmartFilter)`

#### Scenario: A disabled filter group still declines

- **WHEN** the same layer has its filter group switched off but the filter itself
  enabled
- **THEN** the eligibility check returns `Err(GpuError::UnsupportedSmartFilter)`

#### Scenario: Disabled filters stay eligible

- **WHEN** the eligibility check runs on the same document with every filter's
  own enabled flag off
- **THEN** it accepts the document

## MODIFIED Requirements

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
