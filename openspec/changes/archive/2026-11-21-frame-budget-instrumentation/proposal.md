# Proposal

## Why

The existing `PICTURA_METRICS` reports explain committed edits and aggregate phases. Issue [#941](https://github.com/KookaStudio/KookaPictura/issues/941) needs per-canvas-paint timings to find slow frames during the pan, zoom, and rotate workloads in ARCH-013.

## What Changes

- Capture document canvas paints through the existing `PICTURA_METRICS` opt-in, excluding explicit control screenshot renders. Use the end of the Qt canvas paint as the measurement boundary.
- Report input handling, visible tile gather, Qt draw time, measured CPU budget slack against 16.6 ms, and the interval between canvas paints. Report the sum of timestamped GPU blend-pass intervals when supported and attributable to the pixels drawn by that paint. Each interval excludes later conversion, readback, and presentation. Display presentation timing is unavailable at this boundary.
- Add a bounded history of frame records to the existing `metrics` control method so a profiling run can read or reset them in one call.
- Document how to collect the five-second, 24 MP gesture measurements. The change adds diagnostics and leaves the rendering path and frame pacing as they are.

## Capabilities

### Modified Capabilities

- `verification/performance-metrics` adds opt-in canvas frame measurements and defines phase boundaries, CPU slack, and GPU attribution.
- `interop/agentic-control` adds a bounded frame history and resets it through the existing `metrics` method.

## Impact

- The Qt canvas times viewport input and paints. The Rust app metrics accumulator stores frame records and publishes them to control clients.
- Existing timed GPU composite results can provide optional samples through the display tile or image path. The Qt Widgets backing store remains the presentation path.
- `DEVELOPING.md` will document the capture. The change adds no dependencies.
