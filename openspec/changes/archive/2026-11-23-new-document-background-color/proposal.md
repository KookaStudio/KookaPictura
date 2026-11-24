# Proposal

Tracking issue: #1546.

## Why

The New Document dialog's Background Contents list has no color control beside
it, so the initial background can only be white, black, or the toolbox
Background Color. Photoshop CS6 shows a swatch there, and the list has drifted
from the documented White / Background Color / Transparent set by carrying
Black and an always-visible Custom… entry.

## What Changes

- Add a color swatch button to the right of Background Contents, using the
  shared `ColorSwatchWidget`.
- Reduce the list to White, Background Color, and Transparent; Custom is added
  and selected only after the swatch returns a custom color.
- The swatch shows the current choice's fill color, opens the Color Picker from
  any enabled choice, and is disabled while Transparent is selected.
- Picking a color makes it the new document's initial locked Background fill.
- Qt Tests cover the list, the swatch's color and enabled state, an accepted
  pick, a cancelled pick, and the resulting document fill.

## Capabilities

### Added Requirements

- `document/document-lifecycle`: the New Document dialog shows a background
  color swatch whose state follows Background Contents and whose pick sets the
  initial background color.

## Impact

- `crates/pictura-app/cpp/new_document_dialog.h` and `.cpp`
- `crates/pictura-app/cpp/tests/tst_new_document.cpp`
- `translations/pictura_de.ts` and `pictura_fi.ts`
