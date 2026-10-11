# Proposal: add-shape-blur

## Why

Issue #337 (part of the #167 remaining-filter audit; umbrella #183).
`Filter ▸ Blur ▸ Shape Blur` is in CS6's Blur submenu but the app had no entry
for it and the engine no kernel; photorust has no source to port, so this is a
new engine. The docs contract (`docs/06-filters/blur-filters.md`) describes it
as a convolution whose kernel is a chosen Custom Shape preset, with Radius
setting the kernel size, and lists it under the selection-edge caveat
(it samples outside the selection, like Gaussian, Box and Motion Blur).

## What Changes

- `pictura-core` exposes `custom_shape_kernel(index, size)`: a Custom Shape's
  antialiased coverage filling a square edge to edge (the picker preview keeps
  its 10% inset).
- `pictura-filters` adds `Filter::ShapeBlur { shape, radius }`: a true
  convolution with that coverage, `2r+1` px across, normalized by its sum,
  clamp-to-edge, colour planes only. Radius 0 is a no-op; an unknown shape or
  a radius above 1000 is refused.
- `pictura-app` maps a `shape-blur` kind of two slots (Radius, Shape index),
  adds `Filter ▸ Blur ▸ Shape Blur…` after Box Blur, and gives the shared
  filter dialog a `FilterControl::Shape` grid of the Custom Shape catalogue.
- Ceilings: the shape set is the app's six built-in Custom Shapes, not CS6's
  library (Adobe's artwork); the dialog has no large "Shape:" swatch and no
  gear menu for loading shape sets.

## Capabilities

### New Capabilities

- `imaging/blur-filters` gains *Shape blur*: the kernel, its orientation and
  weights, the no-op and refusals, and the ImageMagick oracle.
- `imaging/filter-app-ui` gains *Shape Blur dialog*: the menu entry, the
  radius range and default, the shape grid, the slot order, the preview, and
  Last Filter.

## Impact

- `pictura-core` (`shape/custom.rs`), `pictura-filters` (`blur/shape.rs`,
  `Filter`, `apply`, oracle, digests, profile), `pictura-render`
  (`filter_preserves_opacity`, `preview_apron`), `pictura-app`
  (`filter_map.rs`, split `filter_map_choices.rs`, `filter_commands.{h,cpp}`,
  `filter_param_controls.{h,cpp}`, `command_tree.cpp`), `tst_shape_blur`,
  `scripts/filter_oracle.py`.
