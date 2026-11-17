# Proposal

## Why

The engine's blend-mode table offers all 27 layer modes in every document, but
CS6 restricts the menu by colour mode and bit depth: a 32-bit (HDR) document
offers 15 modes, a Lab document excludes 8, and Bitmap/Indexed documents have
only Normal. The docs→code parity audit records this as LAY-11 (`#589`): the
capability exists only in the contract, so today every blend-mode control can
create a state CS6 cannot, and the Fade dialog already carries a hand-rolled
six-mode Lab list that disagrees with the documented eight.

## What Changes

- Add one engine availability predicate to `pictura-core`: whether a layer
  blend mode is offered for a document's colour mode and bit depth.
- Expose the allowed key list for the active document through the app bridge.
- Filter every layer blend-mode authoring control by that list: the Layers
  panel combo, the New Layer / New Group dialog, the Layer Style dialog's
  layer mode and all effect modes, and the Fade dialog (which already gates,
  but through its own copy).
- Keep a current mode that is unavailable in the document visible as a
  selected, disabled entry rather than silently showing Normal; rebuild the
  filtered list when the document's gate changes.
- Correct the stale 32-bit counts in `docs/05-layers/blend-modes.md` (five
  occurrences; 14 -> 15 and 13 -> 12) and update the 32-bit/Lab "refuse or
  convert" edge case to the interim disabled-current-mode policy pending #490,
  matching the mode list the same document and `docs/04-image-ops/32-bit-hdr.md`
  already print.

Not in scope: save validation or refusal for files that already store an
unavailable mode (the disabled-current-mode policy is interim; #490 owns
refusal/conversion), the Layers-filter Mode list (a query, not an edit), Apply
Image, and group `Pass Through`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/blend-modes`: adds the document-dependent availability
  requirement — 32-bit offers 15 modes, Lab excludes 8, Bitmap/Indexed offer
  Normal only — requires every layer blend-mode control to follow it, including
  the disabled display of an unavailable current mode, and updates the engine
  list requirement's item-data/index semantics for the filtered controls.
- `imaging/filter-fade`: the Fade mode menu follows the shared availability
  table, so Lab disables the documented 8 modes (not 6) and 32-bit documents
  disable the same 12 modes as the layer menus; the control stays shown but
  disabled.

## Impact

- `crates/pictura-core`: the blend-mode availability predicate and its tests.
- `crates/pictura-app`: the key-list bridge, the Layers panel, New Layer, Layer
  Style, and Fade; Qt Test coverage in `tst_blend_mode_list` and `tst_fade`.
- `docs/05-layers/blend-modes.md`: the five stale 32-bit count occurrences
  corrected and the 32-bit/Lab edge case updated to the interim
  disabled-current-mode policy pending #490.
- No new dependencies and no new source files; the C++ helper extends
  `blend_modes.cpp` so `layers_panel.cpp` stays under its file-size ceiling.
