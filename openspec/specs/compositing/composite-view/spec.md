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

### Requirement: A visibility edit refreshes every changed node

A visibility edit that sets exactly a listed set of paths visible and hides every
other node (solo, its restore, and Show All Layers) SHALL refresh the union of the
display reach of every node whose visibility changes, not the reach of the listed
paths. When a changed node's reach cannot be bounded, the edit SHALL fall back to
a full recomposite. The present at every cached pyramid level MUST equal a full
recomposite of the document after the edit, and a lazily opened file's merged
preview MUST be dropped before the refresh.

#### Scenario: Solo hides the unlisted nodes everywhere [cv_solo_hide]

- **WHEN** Alt-click solo keeps a small ordinary layer visible and hides the
  Background and every other layer
- **THEN** the canvas at the selected present level equals a full recomposite,
  including the pixels outside the soloed layer where the Background was

#### Scenario: Restoring the solo snapshot shows the hidden nodes everywhere [cv_solo_restore]

- **WHEN** the solo snapshot is restored after an Alt-click solo
- **THEN** the canvas at the selected present level equals a full recomposite
  with the previously hidden nodes visible again

#### Scenario: Hiding at fit zoom updates the selected level [cv_hidden_at_fit]

- **WHEN** a layer, including the Background, is hidden while the canvas presents
  a reduced pyramid level at fit zoom
- **THEN** the presented crop equals a full recomposite at that level, and a
  lazily opened file no longer shows the merged preview's layer
