# Proposal

## Why

Single Row and Single Column Marquee are documented but missing from the tool registry, so users cannot select a one-pixel band with a click (#525).

## What Changes

- Add both tools to the marquee flyout, with no default shortcut; preserve the rectangle/ellipse shortcut cycle and allow flyout and Alt-click discovery.
- Create a full-width row or full-height column at the clicked image pixel, with selection modes and tool-time feather.
- Show the appropriate options and integrate selection movement, context menus, history and existing selection infrastructure.

## Capabilities

### New Capabilities

- `tools/single-line-marquee`: Discovery, options and click selection behavior for single-row and single-column marquees.

### Modified Capabilities

- `tools/tool-framework`: Exclude unassigned marquee tools from keyboard cycling while retaining flyout cycling.

## Impact

Tool IDs, catalog, registry, flyout, options bar, selection interaction and Qt tests in `pictura-app`; existing selection-mask rasterization and feathering are reused. No new dependencies.
