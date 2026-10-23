# Design

`buildCropGridMenu` builds the grid menu from plain `QAction`s with no check
state, so a click picks the entry and the hover highlight is the only marking.
Qt's stylesheet style places menu icons by the `::icon` offset and ignores the
item padding for them, so `theme.cpp` sets `QMenu#optionsCropGridMenu::icon {
left: 10px; }` to bring the icons in to the Settings menu's checkbox inset
(12 px), and the item's left padding moves only the text. The button icon
re-syncs on `cropOptionsChanged`.
