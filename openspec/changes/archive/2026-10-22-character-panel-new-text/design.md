# Design

`TextSession` gains `newTextCharacter(CharacterSetting&, QString& family)` and
`setNewTextCharacter(const CharacterSetting&, const QString& family)`. Both
return false unless a non-mask session is setting new text. The type handler
keeps an `std::optional<CharacterSetting>` override for the session, reset when
a session opens or ends. `currentSetting()` starts new text from it instead of
the defaults, so the overlay preview and the commit use the same attributes.

The options bar owns family, size, colour, and anti-aliasing through the shared
`TypeOptions`, and `currentSetting()` lays those over the character set. So
`setNewTextCharacter` writes them into `TypeOptions` as well. An active session
only refreshes its overlay on a `TypeOptions` change, so this records nothing.

The panel asks the frame-provided session source first in `refresh` and
`apply`, and falls back to the selected type layer. The frame refreshes the
panel on `ToolController::textEditingChanged`, so it switches to the session
when one opens and back to the layer when it closes.
