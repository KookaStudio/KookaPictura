# Spec Delta

## MODIFIED Requirements

### Requirement: GPU compute is the default compositing backend

The system SHALL composite documents through
`pictura_render::composite(CompositeRequest { doc, region, gpu_enabled }) -> (PixelBuffer, Backend)`.
When `gpu_enabled` is true and `pictura_render::gpu_available()` reports a
usable adapter on the backend selected by `compositing/gpu-backend-selection`,
`composite` SHALL return the GPU composite and `Backend::Gpu`; otherwise it
SHALL return the CPU composite and `Backend::Cpu`. The returned buffer MUST
match `pictura_render::composite_rgba(doc)` within ±1 LSB per channel regardless
of the chosen backend.

#### Scenario: Adapter present and GPU enabled

- **WHEN** a document is composited with `gpu_enabled` true on a machine with a usable adapter on the selected backend
- **THEN** `composite` returns `Backend::Gpu` and a buffer within ±1 LSB of `composite_rgba(doc)`

#### Scenario: GPU disabled or unavailable

- **WHEN** `gpu_enabled` is false, or no usable adapter exists on the selected backend
- **THEN** `composite` returns `Backend::Cpu` and a buffer byte-equal to `composite_rgba(doc)`
