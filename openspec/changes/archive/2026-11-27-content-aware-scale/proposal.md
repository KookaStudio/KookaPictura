# Proposal

## Why

The Edit menu exposes an inert Seam-Aware Scale command (#528). Users cannot resize a raster layer while preferentially preserving important content or an alpha-channel region.

## What Changes

- Add an interactive Content-Aware Scale session with bounding-box and numeric positioning/scaling controls, Amount and Protect channel selection, commit and cancel.
- Implement a deterministic CPU seam-carving approximation with forward energy, protection bias, and a blend with ordinary scaling. Do not claim Adobe algorithmic parity.
- Restrict the command to supported raster targets: refuse layers with attached masks, converted-mode layers without retained source channels, and selected-pixel operations above 8-bit depth. Preserve undo/redo and document state on cancellation or refusal.
- Keep the implementation in separate modules; make only necessary shared registrations. Puppet Warp (#527) and vector transforms (#530) remain separate.

## Capabilities

### New Capabilities

- `tools/content-aware-scale`: Session, target eligibility, options, raster results and history behavior for Content-Aware Scale.

### Modified Capabilities

None.

## Impact

Rust raster operations and document integration, the Qt command and options bar, and their tests are affected. No new dependency is planned. Skin-tone protection requires an unsourced detector and is outside this change; its UI must not imply support. Fixed work and memory limits bound the approximation and can refuse large requests.
