# Proposal

## Why

While a type session is open, the Character and Paragraph panels can miss the text being
edited and write to a layer instead (#1137). On a reopened layer the Character panel edits
the hidden layer, adds a history state, and loses size, colour, anti-aliasing and family at
commit. The Paragraph panel has no session handling at all, and Type Mask sessions edit
whichever type layer is selected.

## What Changes

- **Sessions.** The Character and Paragraph panels edit any open type session: new text, a
  reopened layer, or a Type Mask.
- **Menu actions.** Reset Character, Reset Paragraph and Justification edit the session too,
  and are enabled during one with no type layer selected.
- **Options bar.** The panels re-read the session after an options bar change, so a later
  panel edit keeps the bar's value.

## Capabilities

### Modified Capabilities

- `ui/character-paragraph-panels`: adds that the panels edit every open type session.

## Impact

`TextSession` in `tool_sessions.h`, the type tool, both panels, and their wiring in
`frame_build.cpp`.
