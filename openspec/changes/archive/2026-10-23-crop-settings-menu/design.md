# Design

## State

`CropSettings` (in `tool_options.h`) holds the menu's values beside
`CropOptions::classicMode`, which keeps its own session key. The settings
persist through `tool_options_json.cpp` under `crop`. Leaving Classic sets
Show Cropped Area and Auto Center Preview on; entering it keeps their stored
values but the menu shows them unchecked and disabled.

## Shield

`ImageView::CropShield` carries the style the handler pushes from `show()`.
`cropShieldAlpha()` is the single place the dim level is decided: opaque when
the outside area is hidden, zero when the shield is off, else the opacity,
halved while a box drag or rotation is in progress under Auto Adjust Opacity.
Match Canvas resolves to the view's canvas colour at paint time.

ponytail: Show Cropped Area off hides the outside by painting it opaque in the
shield colour. It does not read a stored crop region, because the engine keeps
none; a re-editable non-destructive crop is #544.

## Auto Center Preview

In Modern mode, when a gesture settles (`recordState`), the view pans so the
box centre is the viewport centre. Zoom is left alone.

## Keys

`P` and `H` are taken in `frame_keys.cpp` before the command-key dispatch,
which would otherwise read them as the Pen and Hand tools. The menu actions
carry no `QKeySequence`; the shortcut column is the action text after a tab.
