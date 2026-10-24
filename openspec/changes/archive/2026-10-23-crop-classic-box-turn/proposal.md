# Proposal

## Why

Review of the Settings menu (#104): in Classic mode rotating the crop box
turned the whole image behind a fixed box, where pre-CS6 Photoshop turns the
box over the fixed image. The menu still read as flat, with a combo for
Opacity and menu-style rows rather than checkboxes, and Custom shield colour
opened Qt's stock colour dialog instead of the project's picker.

## What Changes

- Classic mode shows a rotated box over an unrotated image; the box's handles,
  guides and shield follow it, and a press, drag or hover on the turned box is
  hit-tested in its own frame. Modern keeps turning the content.
- The Settings menu is a panel of real checkboxes with the shortcut letters in
  a right-hand column; Opacity is the shared slider field.
- Custom shield colour opens the project's Color Picker.
- **BREAKING**: none.

## Capabilities

### Modified Capabilities
- `tools/crop-settings`: the Settings menu requirement; Classic box turning and
  the shield colour picker.
