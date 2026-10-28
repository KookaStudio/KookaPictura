## REMOVED Requirements

### Requirement: Undo and re-edit of a generic Smart Filter

**Reason**: generic and Filter Gallery Smart Filters now re-edit (#997); the
requirement said they did not.

**Migration**: see "Undo and re-edit of a Smart Filter" below.

## ADDED Requirements

### Requirement: Undo and re-edit of a Smart Filter

Applying a Smart Filter SHALL record one history state that undo removes. A
generic Smart Filter whose filter has dialog parameters, and a Filter Gallery
Smart Filter, SHALL re-edit: when other Smart Filters are stacked above it the
system SHALL first show CS6's notice that they will not preview during the edit,
with a Don't show again choice that persists; then the filter's dialog (or the
Filter Gallery with the stored stack) SHALL open prefilled. While it is open
the preview SHALL show the chain up to the edited filter with the new values,
and not the filters above it. OK SHALL replace the entry in place, keeping its
position, its enable flag, and the filter mask, as one history state; Cancel
SHALL leave the chain unchanged. A filter with no dialog parameters SHALL NOT
re-edit. Camera Raw and Lens Correction keep their own re-edit.

#### Scenario: Undo removes the filter [sfa_undo]

- **WHEN** `tst_smart_filter_apply::undoRemovesSmartFilter` applies a filter to
  a Smart Object and undoes
- **THEN** the object carries no Smart Filter

#### Scenario: Generic and gallery entries re-edit [sfa_reedit_kind]

- **WHEN** `pictura-app::edit_kind_reports_generic_and_gallery` and
  `tst_smart_filter_apply::editSmartFilterEnabledForGenericEntry` ask for the
  edit kind of a generic filter, a gallery stack, and a parameterless filter
- **THEN** they report a generic dialog, the Filter Gallery, and none, and the
  stored kinds and slots read back for the prefill

#### Scenario: The preview leaves out the filters above [sfa_reedit_preview]

- **WHEN** `pictura-app::edit_preview_hides_filters_above` re-edits the lower
  of two filters, previews new values, and cancels
- **THEN** the preview chain holds only the edited filter with the new values,
  and Cancel restores both filters unchanged

#### Scenario: OK replaces the filter in place [sfa_reedit_commit]

- **WHEN** `pictura-app::edit_commit_replaces_in_place` commits new values for
  the lower of two filters with a selection active
- **THEN** the chain still holds two filters, the lower carries the new values,
  the upper is unchanged, and no mask is created

#### Scenario: A double-click reopens the dialog after the notice [sfa_reedit_dialog]

- **WHEN** `tst_smart_filter_apply::doubleClickReopensTheFilterDialog`
  double-clicks a filter row under another filter, ticks Don't show again, and
  OKs the dialog
- **THEN** the notice and the prefilled dialog appear, the choice persists, and
  the chain keeps both filters in order
