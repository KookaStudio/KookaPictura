## Why

The Layers panel has Opacity and Fill controls, but #523's keyboard path to set either value to zero is missing.

## What Changes

- Recognize two quick zero presses for Opacity and two quick Shift+zero presses for Fill.
- Apply the edit to the eligible selected layers in one undo step through the existing Layers controls' transaction.
- Keep typing, pending-transform input, and existing modified command shortcuts available in their own contexts. Recognize the zero key across keyboard layouts.

## Capabilities

### Modified Capabilities

- `ui/layers-panel`: Add the zero opacity and fill keyboard gestures.

## Impact

Qt key handling, Layers panel dispatch, and Qt regression coverage. Thumbnail sizing already shipped in #586.
