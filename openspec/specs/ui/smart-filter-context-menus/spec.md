# ui/smart-filter-context-menus Specification

## Purpose
The Layers-panel context menus for the "Smart Filters" group row and each
smart-filter row.

## Requirements

### Requirement: The Smart Filters group row's context menu

Right-clicking the "Smart Filters" group row SHALL open a menu with exactly
`Disable Smart Filters` (or `Enable Smart Filters` when the group is disabled),
`Delete Filter Mask`, and `Clear Smart Filters`. `Delete Filter Mask` SHALL be
enabled only when the smart object carries a filter mask. Each item SHALL run
its action on the group's layer and refresh the panel.

#### Scenario: The group menu [sfcm_group_menu]

- **WHEN** `tst_layers_smart_filters::smartFiltersGroupMenu` opens the group
  row's menu on a layer with two filters
- **THEN** the items are Disable Smart Filters, Delete Filter Mask, Clear Smart
  Filters, in that order

#### Scenario: Delete Filter Mask follows the mask [sfcm_mask_gate]

- **WHEN** `tst_layers_smart_filters::deleteFilterMaskFollowsTheMask` opens the
  menu on a layer with no filter mask
- **THEN** Delete Filter Mask is disabled

#### Scenario: The group toggles and clears [sfcm_group_actions]

- **WHEN** `tst_layers_smart_filters::menuTogglesAndClearsFilters` chooses
  Disable Smart Filters and then Clear Smart Filters
- **THEN** the group is disabled and then every filter is removed

### Requirement: A smart filter row's context menu

Right-clicking a smart filter row SHALL open a menu with exactly `Edit Smart
Filter Blending Options…` (disabled), `Edit Smart Filter…`, `Disable Smart
Filter` (or `Enable Smart Filter`), and `Delete Smart Filter`. `Edit Smart
Filter…` SHALL be enabled only for a filter whose dialog we can reopen (Camera
Raw, Lens Correction, a generic filter with dialog parameters, or a Filter
Gallery stack); choosing it SHALL reopen that filter's dialog and replace the
entry. Double-clicking a smart filter row SHALL do the same as `Edit Smart
Filter…`.

#### Scenario: The filter menu [sfcm_filter_menu]

- **WHEN** `tst_layers_smart_filters::smartFilterRowMenu` opens a filter row's
  menu
- **THEN** the items are Edit Smart Filter Blending Options…, Edit Smart Filter…,
  Disable Smart Filter, Delete Smart Filter, in that order

#### Scenario: Edit is gated on the filter kind [sfcm_edit_gate]

- **WHEN** `tst_layers_smart_filters::editSmartFilterIsGatedOnKind` opens the
  menu on a camera-raw filter and on an unmodeled filter
- **THEN** Edit Smart Filter… is enabled for the camera-raw filter and disabled
  for the unmodeled one

#### Scenario: Edit reopens the dialog [sfcm_edit]

- **WHEN** `tst_layers_smart_filters::editSmartFilterRequestsTheDialog` chooses
  Edit Smart Filter… on a Lens Correction filter
- **THEN** the Lens Correction dialog opens prefilled and OK replaces the entry

#### Scenario: A double-click edits a generic filter [sfcm_double_click]

- **WHEN** `tst_smart_filter_apply::doubleClickReopensTheFilterDialog`
  double-clicks a generic filter row
- **THEN** that filter's dialog opens prefilled

#### Scenario: The filter toggles and deletes [sfcm_filter_actions]

- **WHEN** `tst_layers_smart_filters::filterRowDisablesAndDeletes` chooses
  Disable Smart Filter and then Delete Smart Filter
- **THEN** that filter is disabled and then removed
