# Design

## Context

`ZoomToolHandler` uses a release-time 1.2 click factor and a marquee; `notifyToolZoom` carries multi-window and resize behavior. `ImageView` is a QWidget painted with QPainter, not an OpenGL drawing canvas. wgpu compute is a separate capability.

## Goals / Non-Goals

**Goals:** Add scrub and continuous hold zoom without regressing existing zoom operations.

**Non-Goals:** Hand, flick and Bird's Eye features (#534); creating a graphics canvas.

## Decisions

- Store Scrubby Zoom with `ZoomOptions`; choose the documented inferred default on. Use incremental widget-space horizontal displacement, an exponential mapping with a named sensitivity constant, and the existing zoom clamp. Consume displacement at either limit so reversing direction responds immediately; fan out the actual clamped zoom ratio through the existing path and preserve resize semantics. Detect movement using the existing click slop. Suppress the marquee in scrub mode; keep click behavior and the off-mode marquee.
- Expose a persisted Animated Zoom preference, initially off, and a separate capability gate for a live canvas able to accept timed zoom updates. The current raster `ImageView` supports it through anchored `setZoom`; GPU compute availability is not this capability. Adobe's OpenGL prerequisite describes its product, not a requirement to use OpenGL for Pictura's timer and raster repaint. Disable the control with an explanation when the capability is unavailable and use discrete zoom when unavailable or the preference is off.
- With the preference and capability enabled, a stationary press starts a hold delay; after the delay a timer advances zoom continuously until release. A short click retains the single existing step, while a hold that has animated gets no extra release step. A drag before animation takes over enters scrub or marquee mode; once animation starts, conflicting zoom or pan input interrupts it cleanly. Zoom in anchors at the press point and modified zoom out at the viewport centre; apply limits, fanout, resize behavior and view-only semantics as for discrete zoom. Expose a deterministic test seam for delay and animation ticks so Qt tests need no wall-clock sleeps. Stop the timer at limits and on release, cancellation, tool deactivation, document/view loss, or replacement; no queued tick may affect either the old or replacement view.
- End scrub gesture state on release, tool deactivation, document/view loss, or cancellation; avoid subsequent updates. Keep all operations in view state.

## Risks / Trade-offs

The exact scrub sensitivity and animation rate are inferred and require visual review. Raster repaint performance can vary; the capability gate allows a safe discrete fallback when timed updates are unavailable. Shared ImageView and General preferences edits need coordination with #534.
