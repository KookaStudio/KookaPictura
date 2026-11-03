# Proposal

## Why

The dialog opened on Panorama every time. A review run meant as Stack Images ran
as Panorama, which on a layer inside another keeps only what overflows the canvas,
and looked like a Stack Images failure.

## What Changes

The dialog restores the last accepted options for the rest of the session. That
Photoshop does so is inferred from a review screenshot.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/auto-blend-layers`: the dialog restores its last options.

## Impact

`crates/pictura-app/cpp/auto_blend_dialog.*`, `frame_menus_align.cpp`.
