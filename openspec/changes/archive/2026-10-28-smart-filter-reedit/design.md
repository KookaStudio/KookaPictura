# Design: smart-filter-reedit

## Row order

The panel builds the filter rows from the last chain index down. A row's path
stays `<layer>/@sf/<index>`, so the eye, the context menu, and delete keep
addressing the chain index; only the drawing order changes.

## Re-edit session

`PictureViewRust::smart_filter_edit` holds `(layer path, index)` between
`smart_filter_edit_begin` and `smart_filter_edit_end`. While it is set:

- a preview on that layer truncates the typed chain to `index` and pushes the
  dialog's record, so the edited filter shows and the filters above it do not
  (the CS6 notice says so); no mask is made from the selection; Cancel restores
  the whole layer from the baseline, as for a new filter;
- a commit restores the preview and replaces the entry through
  `replace_smart_filter_at`: the codec's `replace_smart_filter` keeps the
  entry's position and enable flag and, in a preserved descriptor, its
  `blendOptions` and unmodeled keys, while `Fltr`, `Nm  `, and `filterID` take
  the new record. The filter mask is untouched.

Edit kinds: 1 Camera Raw and 2 Lens Correction keep their own commands; 3 is a
generic record of one kind that has dialog parameters (a parameterless filter
such as Blur has nothing to edit); 4 is a Filter Gallery record.

## The window

`PicturaMainWindow::editSmartFilter` shows the notice when the filter is not
the top of the chain (the session store's `showSmartFilterEditMessage` turns it
off), begins the edit, runs the kind's dialog prefilled from the record (the
Filter Gallery with `setEffects`), commits through the usual commit path, and
ends the edit. The Layers panel's double-click on a filter row emits the same
`smartFilterEditRequested` as Edit Smart Filter….
