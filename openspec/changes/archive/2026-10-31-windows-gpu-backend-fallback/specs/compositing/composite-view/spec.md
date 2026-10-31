# Spec Delta

## MODIFIED Requirements

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
