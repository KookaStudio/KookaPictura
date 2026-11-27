# Proposal

## Why

Freeform Pen cannot trace image edges, leaving the Magnetic Pen workflow unavailable.

## What Changes

- Add the Magnetic option and edge detection controls to Freeform Pen.
- Trace and edit fastening points, then commit an open or closed path in the selected output mode.
- Reuse the existing magnetic edge field as an approximation of the undocumented Adobe algorithm.

## Capabilities

### New Capabilities

- `tools/magnetic-pen`: edge-following Freeform Pen options and interaction.

### Modified Capabilities

None.

## Impact

Pen options, tool interaction, path authoring and Qt tests. No new dependencies.
