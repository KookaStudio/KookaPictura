# Proposal

## Why

The panel is titled `Pictura Metrics`; the menu item should carry the same name.

## What Changes

- `Help ▸ Developer ▸ Debug Toolset` becomes `Help ▸ Developer ▸ Pictura Metrics`.
  The command id `help.developer.debugToolset` is unchanged, so scripts and recorded
  actions keep working.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `ui/debug-toolset`: the menu entry's name.

## Impact

- `pictura-script` catalogue entry and golden; `tst_debug_toolset.cpp`; docs and trackers.
