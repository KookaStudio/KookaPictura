# Proposal

## Why

Owner review on #541: while typing, CS6 lets the pointer leave the text and
drag the whole text object to a new spot. The Type tools committed on any
press away from the text, so placed type could not be nudged into position
without committing and switching to the Move tool.

## What Changes

- While a Type session is open, the pointer outside the text (and outside
  paragraph type's box and handles) shows the Move tool's cursor.
- A drag from there moves the whole text: the origin follows the pointer, and
  the preview, caret, placement point, baseline guides, and box move with it
  live. Nothing is recorded until the commit, which places the type (or
  re-sets the reopened layer) where it was dropped.
- A plain click outside the text still commits, as before.
- **BREAKING**: none.

## Capabilities

- Added `tools/type-session-move`.
