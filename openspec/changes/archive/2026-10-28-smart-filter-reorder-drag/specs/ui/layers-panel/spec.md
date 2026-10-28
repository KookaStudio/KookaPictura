## MODIFIED Requirements

### Requirement: Smart Filters tree rows

When a smart-object layer carries one or more smart filters, the Layers panel SHALL show a `Smart Filters` parent row directly under that layer, with one child row per filter carrying that filter's name, the most recently added filter on top (the reverse of the apply order), as CS6 stacks them. The parent row SHALL expose a visibility toggle for the group enable flag and each child row SHALL expose a visibility toggle for that filter's enable flag. Toggling either SHALL update the composite, record one history state, and persist the flag to the layer's preserved smart-object descriptor so it survives a write and re-read; the synthetic rows SHALL be non-editable and SHALL NOT appear as selectable layers to layer operations. A filter row SHALL drag up or down within its own Smart Filters list and nowhere else; the drop SHALL move that filter in the chain and record one history state, and because the chain applies bottom-up, a filter dragged above another SHALL apply after it, so its effect shows over the lower one. The Smart Filters parent row SHALL NOT drag. Double-clicking a filter row SHALL re-edit that filter instead of renaming it.

#### Scenario: The parent and one child per filter appear

- **WHEN** `tst_layers_smart_filters::treeShowsGroupAndToggles` shows a smart-object layer with a filterFX chain
- **THEN** the panel shows a `Smart Filters` parent with one child row per filter, named for each filter

#### Scenario: The newest filter is on top [lp_sf_order]

- **WHEN** `tst_smart_filter_apply::filterRowsShowNewestFirst` applies Gaussian Blur and then Emboss to a Smart Object
- **THEN** the Emboss row is above the Gaussian Blur row

#### Scenario: Toggling a filter eye updates the composite

- **WHEN** `pictura-app::app_setter_changes_the_composite_and_records_one_state` flips a child row's visibility toggle
- **THEN** that filter's enable flag flips, the composite updates, and one history state is recorded

#### Scenario: Toggling the group eye bypasses the chain

- **WHEN** `pictura-app::disabled_group_bypasses_the_chain` flips the parent row's visibility toggle off
- **THEN** the group enable flag is cleared and the composite falls back to the unfiltered source

#### Scenario: A toggle survives a write and re-read

- **WHEN** `pictura-app::toggles_persist_on_a_preserved_smart_object` writes and reads back a filtered document whose filter or group visibility was toggled
- **THEN** the re-read smart object reports the toggled flag

#### Scenario: Synthetic rows are not real layers

- **WHEN** `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` double-clicks a smart-filter row
- **THEN** the row is not renamed, no layer is added or moved, and the filter's dialog opens instead

#### Scenario: Dragging a filter row reorders the chain [lp_sf_drag]

- **WHEN** `tst_smart_filter_apply::dragReordersSmartFilters` drags the lower of two filter rows above the upper one, then undoes, then drops a filter row on a layer row
- **THEN** the dragged filter moves to the top row and applies last, undo restores the order, and the drop on a layer row is refused
