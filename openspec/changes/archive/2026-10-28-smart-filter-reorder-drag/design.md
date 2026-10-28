# Design: smart-filter-reorder-drag

Filter rows get the drag and drop item flags; the panel's drag-paths provider
lets a `smart-filter` row through while still refusing the other synthetic
rows. The tree's single validator/handler pair now goes through
`LayersPanel::canDropRow` / `dropRow`: a dragged `…/@sf/<i>` path is accepted
only onto a filter row of the same object, and everything else keeps the
layer-move path.

A row's path carries its chain index, and rows are drawn newest (highest index)
on top. Dropping above target `t` puts the filter just after `t` in the chain,
below puts it just before, and onto the row takes its place; the insert index
is then shifted down by one when the filter came from below it, giving the
`to` of `reorder_smart_filters` (remove `from`, insert at `to`). A drop that
lands where it started is refused, so it records no state.
