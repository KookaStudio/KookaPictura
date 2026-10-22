# Design: Placeholder text and the Warp Text icon

## Placeholder

`TypeToolHandler::beginSession` already resets its `TypeTextEdit` for a new
session. A new (not reopened) session of a non-mask type tool resets it to
`kPlaceholderText` and calls `selectAll()` when the flag is on. The edit model
already replaces a selection on insert and on paste, so typing, pasting, and
Backspace behave as on any selection. A dragged box opens its session on press,
so a click and a drag share this path. The Alt-click Paragraph Text Size
dialog opens its session the same way. Type on a path opens through
`beginSession` too, and also gets the placeholder.

The flag is a `std::shared_ptr<bool>` that `ToolController` creates and hands
to each type handler, with `setTypePlaceholderText(bool)`. It is not a
`TypeOptions` field: setting `TypeOptions` re-sets the selected type layer and
records history.

## Preference

`PreferencesDialog::kType` becomes a real page with one checkbox, key
`fillPlaceholderText`, label "Fill New Type Layers With Placeholder Text"
(Photoshop CC's wording). It emits `typePlaceholderTextChanged`. The frame
stores the setting in `SessionState::typePlaceholderText` (JSON key of the
same name, default true), applies it to the tool controller on startup and on
change, and saves the session. `Edit > Preferences > Type` becomes the command
`edit.preferences.type`, and the Help search index lists the page.

## Icon

`assets/icons/options.warpText.svg`: a custom icon on the Lucide grid —
`M7 4h10`, `M12 4v9`, and the arc `M4 19c5-4 11-4 16 0` — registered in
`lucide-map.json` as `custom` and in `pictura.qrc`.
