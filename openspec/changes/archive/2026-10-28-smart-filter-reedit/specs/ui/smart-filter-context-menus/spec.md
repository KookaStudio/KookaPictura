## MODIFIED Requirements

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
