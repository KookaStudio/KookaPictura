# Proposal

## Why

Review of the Crop tool (#104): switching tools with an unapplied crop silently
dropped it, Perspective Crop had no Reset beside Cancel and Apply, and the
guide-overlay menu used different spacing from the Settings menu.

## What Changes

- Leaving the Crop or Perspective Crop tool with a pending crop asks "Crop the
  image?" (Crop / Cancel / Don't Crop).
- Perspective Crop gains a Reset button, shown with Cancel and Apply once the
  quad is complete.
- The guide-overlay menu uses the Settings menu's padding.
- **BREAKING**: none.

## Capabilities

### New Capabilities
- `tools/crop-leave-prompt`: the confirmation, and Perspective Crop's Reset.
