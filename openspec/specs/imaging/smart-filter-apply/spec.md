# imaging/smart-filter-apply Specification

## Purpose
Applying a Filter-menu filter or a Filter Gallery stack to a Smart Object as a
non-destructive Smart Filter: the Pictura-only generic record, the selection as
the filter mask, the disabled commands, the dialog preview, undo, and save.

## Requirements

### Requirement: A filter on a Smart Object becomes a Smart Filter

Applying a Filter-menu filter to an embedded Smart Object SHALL append a
non-destructive Smart Filter carrying the filter's kind and full-resolution
parameters, and SHALL NOT change the layer's proxy pixels. Applying the same
filter again SHALL append a second entry rather than replace the first. Last
Filter SHALL append in the same way. A linked or legacy placed Smart Object
SHALL refuse the filter, and no destructive filter path SHALL write a Smart
Object's pixels.

#### Scenario: A filter appends a Smart Filter [sfa_append]

- **WHEN** `pictura-render::apply_appends_and_keeps_proxy` and
  `tst_smart_filter_apply::filterMenuAppendsSmartFilter` apply Gaussian Blur
  twice to an embedded Smart Object
- **THEN** the object carries two Smart Filter entries, the proxy pixels are
  unchanged, and the composite shows the blur

#### Scenario: Last Filter appends [sfa_last_filter]

- **WHEN** `pictura-app::last_filter_appends` applies a filter to a Smart
  Object and re-applies it as Last Filter
- **THEN** the object carries two entries

#### Scenario: An ineligible object is refused [sfa_refuse]

- **WHEN** `pictura-render::refuses_ineligible` and
  `pictura-app::pixel_target_refuses_smart_object` apply a filter to a linked
  Smart Object, a legacy placed layer, or a raster layer through the smart op,
  and through the destructive path to a Smart Object
- **THEN** each call is refused and the layer is unchanged

### Requirement: The selection becomes the filter mask

When a Smart Filter is applied with an active selection and the object has no
filter mask, the selection's coverage SHALL become the filter mask and the mask
SHALL be enabled. An object that already has a mask SHALL keep it. Without a
selection no mask SHALL be created.

#### Scenario: A selection becomes the mask [sfa_mask_from_selection]

- **WHEN** `pictura-render::selection_becomes_mask` and
  `tst_smart_filter_apply::selectionBecomesFilterMask` apply a filter with the
  left half selected
- **THEN** the object's filter mask is white over the left half and black
  elsewhere, enabled, and the composite filters only the left half

#### Scenario: An existing mask is kept [sfa_mask_kept]

- **WHEN** `pictura-render::existing_mask_kept` applies a filter with a
  selection to an object that already has a mask
- **THEN** the mask is unchanged

#### Scenario: No selection makes no mask [sfa_no_selection]

- **WHEN** `pictura-render::no_selection_no_mask` applies a filter with no
  selection
- **THEN** the object has no filter mask

### Requirement: The generic Smart Filter record

The system SHALL store a Smart Filter it does not model as a Pictura-only
record (`filterID` "PTFL") whose options list the steps' catalogue kinds and
slots and whether the entry is a Filter Gallery stack, and whose name is the
catalogue label without its ellipsis, or "Filter Gallery". Rendering the record
SHALL produce the same pixels as the destructive filter over the same source.
A record naming an excluded or unknown kind, or a malformed record, SHALL be
undecodable, so the layer falls back to its proxy.

#### Scenario: The record round-trips [sfa_record_round_trip]

- **WHEN** `pictura-codec::steps_round_trip` and
  `pictura-codec::gallery_flag_round_trips` encode a stack and decode it
- **THEN** the kinds, slots, and gallery flag are unchanged

#### Scenario: A malformed record decodes to nothing [sfa_record_malformed]

- **WHEN** `pictura-codec::malformed_options_decode_none` decodes an empty
  buffer, a record with no steps, and one with a wrongly typed key
- **THEN** each decodes to `None`

#### Scenario: The record renders like the destructive filter [sfa_render_matches]

- **WHEN** `pictura-render::generic_record_matches_destructive_apply` renders a
  record of Gaussian Blur over a buffer with transparency
- **THEN** the pixels equal the destructive Gaussian Blur of the same buffer

#### Scenario: Gallery steps apply in order [sfa_gallery_order]

- **WHEN** `pictura-render::gallery_steps_apply_in_order` renders a two-step
  gallery record
- **THEN** the pixels equal the two destructive filters applied in order

#### Scenario: An excluded kind is undecodable [sfa_excluded_undecodable]

- **WHEN** `pictura-render::excluded_kind_is_undecodable` decodes a record
  naming Liquify
- **THEN** the record does not decode

### Requirement: A Filter Gallery stack is one Smart Filter

Committing a Filter Gallery stack on a Smart Object SHALL append exactly one
Smart Filter named "Filter Gallery" holding every visible effect in order.

#### Scenario: The gallery adds one entry [sfa_gallery_one_entry]

- **WHEN** `pictura-app::gallery_commits_one_entry` and
  `tst_smart_filter_apply::filterGalleryAddsOneEntry` commit a two-effect
  gallery stack on a Smart Object
- **THEN** the object gains one "Filter Gallery" entry carrying both steps

### Requirement: Commands a Smart Object cannot take are disabled

On an embedded Smart Object, the Filter menu SHALL disable Liquify, Vanishing
Point, Lens Blur, Displace, and Adaptive Wide Angle, and a direct request for
one SHALL be refused with a message to rasterize the object. Image >
Adjustments SHALL be disabled. A layer-mask target SHALL keep both enabled.

#### Scenario: Excluded filters are disabled [sfa_excluded_disabled]

- **WHEN** `pictura-filters::smart_capable_excludes_cs6_list`,
  `pictura-filters::resource_kinds_are_not_smart_capable`, and
  `tst_smart_filter_apply::excludedFiltersDisabledOnSmartObject` check the
  catalogue and the Filter menu over a Smart Object
- **THEN** the five kinds are not smart-capable, every resource-reading kind is
  excluded, and their menu entries are disabled while Gaussian Blur is enabled

#### Scenario: Adjustments are disabled [sfa_adjustments_disabled]

- **WHEN** `tst_smart_filter_apply::adjustmentsDisabledOnSmartObject` selects a
  Smart Object
- **THEN** Image > Adjustments entries are disabled, and enabled again on a
  raster layer

### Requirement: The dialog preview is non-destructive

A filter dialog's preview on a Smart Object SHALL show the filter as a Smart
Filter on top of the chain without changing the document; Cancel SHALL restore
the layer exactly, and OK SHALL append exactly one entry.

#### Scenario: Cancel restores the layer [sfa_preview_cancel]

- **WHEN** `pictura-app::preview_cancel_restores_layer` and
  `tst_smart_filter_apply::previewCancelRestoresSmartObject` preview a filter
  on a Smart Object and cancel
- **THEN** the layer, its Smart Filters, and its mask equal the originals

#### Scenario: Preview then commit appends once [sfa_preview_commit]

- **WHEN** `pictura-app::preview_then_commit_appends_once` previews twice then
  commits
- **THEN** the object carries exactly one new entry

### Requirement: Applied Smart Filters survive a save

A Smart Object's applied Smart Filters and filter mask SHALL survive a save and
reopen in Pictura. Opening the file in Photoshop is unverified.

#### Scenario: Save and reopen keep the filters and the mask [sfa_save_reopen]

- **WHEN** `pictura-render::save_reopen_keeps_filters_and_mask` applies two
  filters with a selection, writes the document, and reads it back
- **THEN** the re-read object carries both entries, the same mask, and
  composites the same pixels

#### Scenario: Photoshop opens the authored Smart Filters [sfa_photoshop]

- **WHEN** a document saved with a generic Smart Filter and a mask is opened in
  Photoshop
- **THEN** the mask is shown; the generic entries are not expected to render

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
