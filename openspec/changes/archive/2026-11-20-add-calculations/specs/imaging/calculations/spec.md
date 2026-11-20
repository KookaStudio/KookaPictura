# Spec Delta

## Purpose

Image ▸ Calculations: blend two individual channels from one or more open
documents, through a blend mode, opacity, and an optional mask, into a new
grayscale document, a new alpha channel, or the selection.

## ADDED Requirements

### Requirement: Calculations dialog

`Image ▸ Calculations…` SHALL open a modal dialog with Source 1 and Source 2
groups (Image, Layer, Channel, Invert), Blending, Opacity, an optional Mask
group with an enable toggle (Image, Layer, Channel, Invert), Result
(`New Document`, `New Channel`, `Selection`), a Preview checkbox, and a result
preview.

Blending SHALL default to Multiply, Opacity to 100 %, and Result to
`New Document`. Scale (`1.000`–`2.000`, default `1.000`) and Offset
(`−255`–`255`, default `0`) SHALL be shown only for Add and Subtract.

The command SHALL be disabled without a document and on Indexed and Bitmap
documents.

#### Scenario: Sources must match the active document's dimensions [cal_sources_same_size]

- **WHEN** another open document has different pixel dimensions from the active
  document
- **THEN** it is absent from both Source lists, and the active document is
  listed first

#### Scenario: Composite is absent and Gray is offered [cal_channel_list]

- **WHEN** a Source or Mask Channel list is read for any colour mode
- **THEN** it offers `Gray`, the colour planes, Transparency, the layer mask
  when the chosen layer has one, and the alpha channels, and it does not offer
  the composite channel

#### Scenario: Scale and Offset follow the mode [cal_scale_offset_rows]

- **WHEN** Blending is Add or Subtract
- **THEN** Scale and Offset are shown, and they are hidden for every other mode

#### Scenario: Defaults [cal_defaults]

- **WHEN** the dialog opens
- **THEN** Blending is Multiply, Opacity is 100 %, Result is New Document, and
  Add/Subtract rows are hidden

#### Scenario: The mode list [cal_mode_list]

- **WHEN** the Blending list is read
- **THEN** it holds the layer modes without Dissolve, Multiply is selected, and
  Subtract appears exactly once

#### Scenario: Disabled modes and no document [cal_disabled_modes]

- **WHEN** the active document is Indexed or Bitmap, or no document is open
- **THEN** the Calculations command is disabled

#### Scenario: Menu row is live [cal_menu_row]

- **WHEN** a document is open and the Image menu is read
- **THEN** the Calculations row is enabled and opens the dialog

### Requirement: Calculations blending

Calculations SHALL reduce each source channel to one grayscale value per pixel
and blend Source 1 (A) with Source 2 (B) using the chosen mode. A single
channel SHALL feed the blend as its own value, and `Gray` SHALL be the Rec.601
grayscale conversion of the chosen layer or merged image.

Invert SHALL use a source's complement before blending. Opacity SHALL mix the
blended value toward A. Add SHALL compute `(A + B) / Scale + Offset` and
Subtract `(A − B) / Scale + Offset`, where Offset is `offset / 255` in the unit
domain. The operation SHALL run in the unit `f32` domain at every source depth
before any 8-bit narrowing.

#### Scenario: Add with Scale 2 averages [cal_add_scale_two]

- **WHEN** Add runs with Scale 2 and Offset 0
- **THEN** each result is the average of the two source values within 1/255

#### Scenario: Subtract honours Scale and Offset [cal_subtract_scale_offset]

- **WHEN** Subtract runs
- **THEN** a higher Scale reduces the result and a positive Offset raises it

#### Scenario: Layer modes use the shared blend [cal_layer_modes]

- **WHEN** each listed layer mode runs on the two sources
- **THEN** the result equals the compositor's blend function for that mode

#### Scenario: Invert applies per source [cal_invert]

- **WHEN** Source 1 Invert, Source 2 Invert, or both are set
- **THEN** the same blend runs against the complemented source values

#### Scenario: Opacity mixes toward Source 1 [cal_opacity]

- **WHEN** Opacity is 50 %
- **THEN** each result is halfway between Source 1's value and the full blend

#### Scenario: Gray matches the grayscale conversion [cal_gray_luminance]

- **WHEN** a source channel is Gray
- **THEN** its value equals the Rec.601 grayscale conversion of the chosen layer
  or merged image

#### Scenario: Alpha channel as a source [cal_alpha_source]

- **WHEN** a Source channel is a saved alpha channel
- **THEN** the blend reads its coverage as the source value

#### Scenario: 16-bit sources keep blend precision [cal_16bit_precision]

- **WHEN** a 16-bit source differs from another by less than one 8-bit step
- **THEN** that difference can change the narrowed 8-bit result

### Requirement: Calculations mask

The optional Mask group SHALL accept an image, layer, and channel, or
Transparency, or the Mask image's selection (the active document's selection
when the Mask image is the active document), with Invert. The Selection entry
SHALL be absent when the Mask image has no selection. The mask value SHALL
scale the result toward black: white reveals, black hides. With no mask, the
result applies everywhere.

#### Scenario: Mask hides [cal_mask_hides]

- **WHEN** a mask channel is black at a pixel
- **THEN** the result is black there

#### Scenario: Inverted mask [cal_mask_invert]

- **WHEN** the mask is inverted
- **THEN** the blend shows where the mask channel is dark

#### Scenario: Selection as mask [cal_selection_mask]

- **WHEN** the mask channel is Selection
- **THEN** the result scales by the selection coverage and is black outside it

#### Scenario: No mask applies everywhere [cal_no_mask]

- **WHEN** no mask is chosen
- **THEN** the whole document holds the unmodified blend

### Requirement: Calculations results

The Result control SHALL send the same blended plane to one of three targets.

`New Document` SHALL open a new 8-bit grayscale document of the active
document's size whose pixel layer holds the result; the source document SHALL
gain no history state.

`New Channel` SHALL append an alpha channel holding the result under the next
default alpha name, as one `Calculations` history state that undo removes. At
the 56-channel cap the command SHALL refuse and report the 56-channel limit to
the user, leaving the document unchanged.

`Selection` SHALL replace the live selection with the result coverage as one
`Calculations` history state that undo restores to the previous selection.

#### Scenario: New Document [cal_result_document]

- **WHEN** Result is New Document and OK is pressed
- **THEN** a new grayscale document of the same size opens, its pixels equal
  the blend, and the source document's history is unchanged

#### Scenario: New Channel [cal_result_channel]

- **WHEN** Result is New Channel and OK is pressed
- **THEN** an alpha channel holding the blend is appended and named as the next
  default alpha channel

#### Scenario: New Channel undo [cal_result_channel_undo]

- **WHEN** a New Channel result is undone
- **THEN** the channel disappears and the document is otherwise unchanged

#### Scenario: New Channel at the cap [cal_result_channel_cap]

- **WHEN** the document already holds the maximum number of channels
- **THEN** the result is refused with the limit reported and no channel is added

#### Scenario: Selection [cal_result_selection]

- **WHEN** Result is Selection and OK is pressed
- **THEN** the selection coverage equals the blend

#### Scenario: Selection replaces [cal_result_selection_replace]

- **WHEN** a selection is live and Result is Selection
- **THEN** the live selection is replaced, and undo restores it

#### Scenario: Results agree [cal_results_agree]

- **WHEN** the same settings are committed once per result target
- **THEN** the new document's pixels, the new channel's coverage, and the
  selection's coverage are equal

### Requirement: Calculations preview and history

The dialog SHALL show a preview image of the result while Preview is checked
and hide it when Preview is unchecked. The preview SHALL reflect the current
controls and SHALL NOT modify the document, its channels, or its selection.

Cancel SHALL leave the document, its channels, and its history unchanged. OK
SHALL commit exactly one result target change.

#### Scenario: Preview follows controls [cal_preview_follows]

- **WHEN** a source, blend, opacity, mask, or invert control changes
- **THEN** the preview image changes without altering the document

#### Scenario: Preview off [cal_preview_off]

- **WHEN** Preview is unchecked
- **THEN** the preview image is hidden and the document is still unchanged

#### Scenario: Cancel changes nothing [cal_preview_cancel]

- **WHEN** the dialog is cancelled after previewing
- **THEN** the document pixels, channels, selection, and history are unchanged

#### Scenario: Preview matches commit [cal_preview_matches_commit]

- **WHEN** OK commits with the current settings
- **THEN** the committed result equals the previewed blend
