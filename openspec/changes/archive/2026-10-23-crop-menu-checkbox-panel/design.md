# Design

Reverts `buildCropSettingsMenu` and its `QMenu#optionsCropSettingsMenu` rules in
`theme.cpp` to the checkbox-panel form: one `QWidgetAction` holding `QCheckBox`
rows, so the checkboxes share the theme's `QCheckBox::indicator` look.
