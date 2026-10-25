# Design: Layers panel smoke-test follow-ups

## Mask thumbnail targeting

`set_layer_target()` drops a mask target whose anchor does not match the new
active path. The mouse filter used to set the target before QTreeView applied
the selection, so a selection clear/reselect sequence could drop it while the
active-thumb border remained. Mask-thumb clicks now select the row through the
selection model first, then set the target, and consume the click. The mask
thumbnail context menu does the same.

Ctrl-click on the mask thumbnail is intercepted before the plain mask branch:
it selects the row and calls `mask_selection(path, "new")`, then consumes the
click. It never reaches the tree's Ctrl-click row toggle.

## Mask-target adjustments

`filter_target_ready()` now returns true when `active_mask_target()` is set.
`apply_op_active_region()` already routes adjustments and filters to
`apply_mask_op()`, so the guard only needed to stop requiring an active pixel
layer. This covers fill, adjustment, and group masks.

## Menu table

The row table keeps its block order and adds:

- Copy CSS/Copy SVG disabled rows in block 1 for type/group/adjustment;
- export for every non-Background kind;
- Artboard/Frame disabled rows and Convert to Smart Object on type rows;
- raster and vector mask rows for adjustment/fill rows, vector masks for groups;
- clipping and merge rows for fill/adjustment;
- a type-only block after rasterize for orientation, anti-aliasing,
  point/paragraph conversion, Warp Text, and disabled work-path/shape rows;
- per-kind Duplicate/Delete labels.

Fill rows are still the `adjustment` kind; `layer_is_fill_content()` separates
them where the CS6 menu differs.

## Group Blending Options

Groups render no effects yet. `set_layer_style_value()` accepts only
`blending.*` for a group; the Layer Style dialog opens on Blending Options and
marks the ten effect rows disabled. `layer_style_can_edit()` now accepts a group
while still rejecting the Background. Full group-effect rendering is #939.

## Disabled placeholders

Commands with no implementation stay visible with the standard
`— not implemented yet` tooltip: Copy CSS, Copy SVG, Artboard from Layers…,
Frame from Layers…, Convert to Frame, Create Work Path, Convert to Shape, the
Windows anti-alias modes, Postcard, and 3D extrusion.
