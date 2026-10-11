# Proposal: expose-custom-filter

## Why

Issue #336 (part of the #167 bundle). The Custom convolution kernel is
already ported (`Filter::Custom` in `pictura-filters`, `imaging/other-filters`),
but nothing reaches it: there is no `custom` kind in the bridge and no
Filter-menu row, so `Filter ▸ Other ▸ Custom` stays a disabled stub. The docs
contract (`docs/06-filters/other-filters.md`) lists Custom as Core parity
alongside High Pass, Maximum, Minimum, and Offset, which are all exposed.

## What Changes

- `pictura-app` maps a `custom` kind of 27 slots: the 5×5 weights row by row,
  then Scale and Offset. An empty list is the pass-through kernel
  (centre 1, Scale 1, Offset 0).
- `Filter ▸ Other ▸ Custom…` opens the shared filter dialog with a 5×5 grid
  of integer cells (`-999..=999`) and Scale (`1..=9999`) and Offset
  (`-9999..=9999`) as plain value boxes, with the usual live preview.
- Two new dialog controls: `FilterControl::Matrix` (a 5×5 grid, 25 slots) and
  `FilterControl::Number` (a value box with no slider).
- Ceiling: the dialog's Save/Load buttons (CS6 `.acf` presets) are not
  built; the docs give no file layout for them.

## Capabilities

### New Capabilities

- `imaging/filter-app-ui` gains *Custom dialog*: the grid, the ranges and
  defaults, the slot order, the preview, and Last Filter.

## Impact

- `pictura-app` (`filter_map.rs`, `filter_map_tests.rs`,
  `filter_commands.{h,cpp}`, `filter_param_controls.{h,cpp}`), plus
  `tst_custom_filter` and the Custom slot count in `tst_filter_menu`.
- No engine change; the kernel and its ImageMagick oracle already exist.
