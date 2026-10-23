# Design

Both Crop menus carry an object name the theme styles together
(`optionsCropSettingsMenu`, `optionsCropGridMenu` in `theme.cpp`), so their
padding, icon gutter and separators cannot drift apart. The Settings menu's
items are `QAction`s again (native check marks); only Color and Opacity are
`QWidgetAction`s, indented by `kCropMenuTextIndent`.
