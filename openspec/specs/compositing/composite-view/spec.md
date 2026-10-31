# composite-view Specification

## Purpose
Displays the composited layer stack, falling back to the embedded composite, with an offscreen GPU path.

## Requirements

### Requirement: Display the composited layer stack when a PSD has layers

When a loaded PSD document contains layers, the Qt app SHALL display the result
of `pictura_render::composite_rgba` for that layer stack rather than the embedded
PSD composite. Regions not covered by any visible layer MUST be transparent, and
changes to layer visibility or adjustment layers MUST refresh the displayed
composite.

#### Scenario: Layered PSD is composited

- **WHEN** a PSD whose document has one or more layers is opened
- **THEN** the displayed image is the composited layer stack, and a pixel outside
  every visible layer's rectangle is transparent (alpha 0)

#### Scenario: Layer edit refreshes the view

- **WHEN** a layer's visibility is toggled or an adjustment layer is added or
  removed
- **THEN** the app recomposites and displays the updated image

### Requirement: Fall back to the embedded PSD composite when there are no layers

When a loaded PSD document has no layers, the app SHALL display the document's
embedded composite (`Document::composite`) instead of an empty layer stack.

#### Scenario: Flat PSD

- **WHEN** a PSD with no layer records is opened
- **THEN** the displayed image is the embedded PSD composite, not a blank image

### Requirement: Offscreen GPU path and headless self-test

The app SHALL render offscreen through wgpu on the backend selected by
`compositing/gpu-backend-selection`, read the result back to the CPU, and wrap
it as a `QImage` for display. `--self-test` MUST run without an on-screen surface
(Xvfb/offscreen), MUST exit 0 when the composed/rendered image is non-blank, and
MUST exit non-zero when it is blank or the composition is wrong. When no usable
adapter is available on the selected backend, the offscreen path MUST report
unavailability and keep the CPU fallback, and the self-test MUST still exit 0.

#### Scenario: Offscreen render is displayed

- **WHEN** the app renders offscreen on a usable adapter on the selected backend
- **THEN** the readback is wrapped as a `QImage` and shown through the existing
  `PictureView` display path

#### Scenario: Self-test passes headless

- **WHEN** `pictura --self-test <layered.psd>` runs under Xvfb or the offscreen
  platform
- **THEN** it reports the composited image and a non-blank result and exits 0

#### Scenario: Self-test without an adapter

- **WHEN** `pictura --self-test` runs with no usable adapter on the selected backend
- **THEN** the offscreen path reports unavailability, the CPU fallback image is
  displayed, and the process still exits 0
