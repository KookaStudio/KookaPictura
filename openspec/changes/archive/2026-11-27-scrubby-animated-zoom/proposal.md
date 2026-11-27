# Proposal

## Why

The Zoom tool supports clicks and marquee zoom but lacks Scrubby Zoom and continuous hold zoom. Animated Zoom is absent from General preferences.

## What Changes

- Add Scrubby Zoom and retain marquee zoom when it is off.
- Add preference-controlled Animated Zoom on supported canvases, including the current raster canvas, with a discrete fallback when disabled or unavailable.
- Preserve multi-window zoom, view-only state, and existing click behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/canvas-tools`: make zoom dragging conditional and add preference-controlled continuous hold zoom.

## Impact

Zoom options, canvas tool handling, General preferences, Qt tests, and feature trackers. No new dependencies. A graphics-backed canvas is outside this change.
