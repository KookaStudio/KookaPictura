# Proposal: The Character panel edits new type as it is set

## Why

With new type now starting as selected placeholder text, owner review of #684
found that Character panel toggles (Faux Bold, Underline, …) did nothing
visible until the text was committed. The panel only edited a committed type
layer. While new text was being set it either edited nothing or, when a type
layer was selected, edited that other layer.

## What changes

- While a Horizontal or Vertical Type session sets new text, the Character
  panel shows and edits that session's attributes. Every change re-renders the
  on-canvas preview at once, and the committed layer carries them.
- The layer selected before the session is not touched.
- Sessions that reopen an existing type layer keep today's path: the panel
  edits that layer, which the preview already follows.

## Capabilities

- Modified (added requirement): `ui/character-paragraph-panels`.

## Impact

- `tool_sessions.h`: two `TextSession` methods. `tool_type.cpp`: a per-session
  character override. `character_panel.{h,cpp}`: a session source used by
  `refresh` and `apply`. `frame_build.cpp`: the wiring.
- Test: `tst_character_paragraph_panels::characterPanelEditsNewText`.
