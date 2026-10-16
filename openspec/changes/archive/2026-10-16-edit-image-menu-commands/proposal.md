# Proposal: edit-image-menu-commands

## Why

Issue #84: the Edit and Image menus carried several `leaf(...)` stubs
(`implemented = false`) with no handler. This change wires the subset ported
from photorust and leaves the rest as documented stubs:

- Edit ▸ Fill… and Edit ▸ Stroke…
- Edit ▸ Purge ▸ Undo / Histories / All (Clipboard was already wired)
- Image ▸ Trim… and Image ▸ Duplicate…

Ported from photorust's `FillDialog.cpp`, `StrokeDialog.cpp`, and the matching
MainWindow handlers.

## What Changes

- `pictura-paint`: `bucket::stroke_selection` bands a selection edge with a
  solid colour (Inside / Center / Outside) using separable max/min filters.
- `cxxqt_object/paint_tools/fills.rs`: the `edit_fill` bridge fills the active
  pixel layer with the foreground colour or a built-in pattern (opacity,
  blend mode, Preserve Transparency) and `edit_stroke` outlines the selection,
  each as one `"Fill"` / `"Stroke"` state.
- `cxxqt_object/impl_history/purge.rs` (new bridge): `purge_all`,
  `purge_history`, and `purge_named_history` over the `History` stack, plus
  `current_history_label`.
- `pictura-app` `History`: `purge`, `purge_states`, and `purge_snapshots`.
- `cxxqt_object/image_adjust/image_ops.rs` (new bridge): `trim_image` crops the
  canvas to the composite's content bounds as one `"Trim"` state, and
  `duplicate_into` / `duplicate_name` copy a document (optionally merged) into
  a new tab as one `"Duplicate"` state.
- C++ dialogs `fill_dialog.*`, `stroke_dialog.*`, `duplicate_image_dialog.*`,
  and the handler wiring in `frame_menus_edit.cpp` (Fill / Stroke / Purge) and
  `frame_menus_image.cpp` (Image ▸ Mode / Crop / Trim / Duplicate).
- `commands.h` / `command_tree.cpp`: `edit.fill`, `edit.stroke`,
  `edit.purge.undo|histories|all`, `image.trim`, `image.duplicate` become
  enabled commands.
- Tests: `bucket` unit tests for `stroke_selection`, the Qt Test
  `tst_fill_tools` (Fill / Stroke), the `tst_edit_clipboard` Purge additions,
  and `tst_image_ops` (Trim / Duplicate).

## Capabilities

### New Capabilities

- `ui/edit-menu-commands`: the Edit ▸ Fill / Stroke / Purge commands and the
  Image ▸ Trim / Duplicate commands.

## Impact

- `pictura-paint`, `pictura-app` (bridge, C++). No new dependency. New
  `.cpp`/`.h` files are added to `CMakeLists.txt` explicitly; the Qt Tests are
  added to `cpp/tests/CMakeLists.txt`.

## Provenance

Source: https://github.com/perfecto25/photorust (`shell/src/dialogs/FillDialog.*`,
`StrokeDialog.*`, and the MainWindow Fill / Stroke / Purge / Trim / Duplicate
handlers).
Co-authored-by: Zawaro <zawaroarts@gmail.com>

Ceiling (`ponytail:`): the stroke band is a square-capped dilate/erode
approximation of CS6's round stroke; Image ▸ Reveal All, Edit ▸ Find And
Replace Text / Define Brush Preset / Define Pattern / Define Custom Shape, and
Type ▸ Create Work Path / Convert to Shape stay disabled stubs.
