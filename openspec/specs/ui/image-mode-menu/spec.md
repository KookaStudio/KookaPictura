# ui/image-mode-menu Specification

## Purpose
Wires the `Image ▸ Mode` submenu onto the mode and bit-depth conversions: check
marks for the current mode and depth, the CS6 prompts, the Indexed Color and
Bitmap dialogs, and one history state per conversion.

## Requirements

### Requirement: The Mode submenu reflects the document

The Bitmap, Grayscale, Indexed Color, RGB Color, CMYK Color, Lab Color,
Multichannel, and 8/16/32 Bits/Channel entries SHALL be checkable commands. The entry for the
document's mode and the entry for its depth SHALL be checked. An entry SHALL be
enabled when the conversion is available or it is the checked entry, and
selecting the checked entry SHALL change nothing. Duotone SHALL remain a
disabled stub. Color Table SHALL follow the depth entries and SHALL be
enabled for an Indexed document with a current editable table. Without a
document every entry SHALL be disabled.

#### Scenario: An RGB document

- **WHEN** an 8-bit RGB document is active
- **THEN** RGB Color and 8 Bits/Channel are checked and enabled, Grayscale, Indexed Color, CMYK Color, Lab Color, Multichannel, and 16/32 Bits/Channel are enabled; Bitmap, Duotone, and Color Table are disabled

#### Scenario: An indexed document

- **WHEN** an Indexed document with a current editable palette is active
- **THEN** Color Table is enabled and opens the document palette

### Requirement: Conversions prompt as CS6 does

Converting a color document to Grayscale SHALL first ask to discard color
information (Discard / Cancel). Converting a layered document to Grayscale,
CMYK, or Lab SHALL offer Flatten / Don't Flatten / Cancel. Converting a layered
document to Indexed Color, Bitmap, or Multichannel SHALL ask to flatten layers
(OK / Cancel), since the conversion flattens. Indexed Color SHALL open a dialog with Palette
(Exact when eligible, Web, Local Perceptual/Selective/Adaptive), Colors,
Forced, Transparency, Dither (None, Diffusion, Pattern, Noise), Amount, and Preview. Preview SHALL show the conversion without recording history, and
Cancel SHALL restore the document exactly. Bitmap SHALL open a dialog offering
50% Threshold, Pattern Dither, Diffusion Dither, Halftone Screen, and Custom
Pattern. Leaving a 32-bit document
SHALL open the HDR Conversion dialog. Cancelling any prompt or dialog SHALL
change nothing.

#### Scenario: Indexed preview then cancel

- **WHEN** the Indexed Color dialog previews a layered document and is
  cancelled
- **THEN** the layers and mode are restored and no history state was added

### Requirement: One Convert Mode state per conversion

Each completed conversion through the `image_mode` bridge (`convert_image_mode`,
`convert_image_depth`, `convert_to_indexed`, `convert_to_bitmap`) SHALL
recomposite and record exactly one history state labelled `Convert Mode`. A
refused conversion SHALL record nothing. Undo SHALL restore the exact prior
document, including its mode and depth.

#### Scenario: Grayscale and undo

- **WHEN** an RGB document is converted to Grayscale and then undone
- **THEN** one `Convert Mode` state was added and the composite pixels equal the
  original RGB

#### Scenario: Multichannel conversion prompts once and can be undone [multi_menu_history]
- **WHEN** a layered RGB document is converted through Image › Mode › Multichannel after accepting the flatten prompt
- **THEN** the checked mode is Multichannel, its three named spot plates are shown, and exactly one Convert Mode state is added
- **AND** undo restores the original mode and layer stack
- **AND** cancelling the prompt leaves the document and history unchanged

#### Scenario: Indexed and 32-bit documents dim Multichannel [multi_menu_gate]
- **WHEN** an Indexed Color or 32-bit document is active
- **THEN** Multichannel is unavailable

### Requirement: Indexed conversion offers Forced and Transparency

The document Indexed Color dialog SHALL offer None, Black And White, Primaries,
Web and Custom forced colors, and a Transparency toggle. Forced colors SHALL
be included within the format's limit. The converter SHALL reserve one entry
for pixels below half opacity when Transparency is selected and matte those
pixels over white when it is off. Preview and acceptance SHALL use the same
options. Existing callers without the new options SHALL keep their defaults.
Every distinct forced color SHALL be retained. If 256 distinct forced colors
and a required clear entry cannot both fit, conversion SHALL reject the request
without changing the document or history.

Exact SHALL be available only when the union of distinct opaque quantization
inputs after the selected transparency and white-matte treatment and distinct
forced colors, plus one entry when a clear pixel requires it, fits 256 entries.
The dialog SHALL recompute availability when options change and switch a selected
Exact palette to Local (Adaptive) if Exact becomes unavailable. A direct invalid
Exact request SHALL be rejected without changing the document or history.

#### Scenario: Convert using forced colors and transparency

- **WHEN** a document with opaque and transparent pixels is converted using Black And White and Transparency
- **THEN** its table contains black, white and one clear entry, and saving and reopening a native PSD preserves its index transparency

#### Scenario: Convert without transparency

- **WHEN** conversion is performed with Transparency cleared
- **THEN** transparent source pixels are composited against white and the result has no transparent index

#### Scenario: Forced colors at the transparency capacity boundary

- **WHEN** a source needing a clear entry is converted with 255 distinct forced colors and then requested with 256 distinct forced colors
- **THEN** the first conversion retains all forced colors and the clear entry, and the second request is rejected without changing the document or history

#### Scenario: Exact availability follows conversion options

- **WHEN** Exact is selected and changing forced colors or Transparency makes the distinct quantization inputs, forced colors and required clear entry exceed 256 entries
- **THEN** Exact becomes unavailable and the dialog selects Local (Adaptive), while an invalid direct Exact request is rejected without changing the document or history

#### Scenario: Exact fits at capacity

- **WHEN** the distinct quantization inputs, forced colors and required clear entry together occupy exactly 256 entries
- **THEN** Exact is available and conversion preserves all required opaque colors and the clear entry

### Requirement: Color Table edits retain pixel indices

The Color Table dialog SHALL support editing entries, interpolating a selected
range, setting or clearing a transparent index, and loading and saving ACT
files. Accepting a changed table SHALL recolor the current indexed image
without remapping or discarding existing pixel indices and SHALL create one
undoable state. Cancelling or accepting an unchanged table SHALL record nothing.

#### Scenario: Change and undo a palette

- **WHEN** a user changes a palette entry and assigns transparency to a different entry, accepts, and then undoes the edit
- **THEN** the indexed pixels retain their indices, their colors and alpha follow the changed table, and undo restores the original table and appearance

#### Scenario: Cancel a table edit

- **WHEN** the user cancels a modified Color Table dialog
- **THEN** the working palette, pixels and history stay unchanged

#### Scenario: Interpolate a palette range

- **WHEN** a user interpolates between two palette entries and accepts
- **THEN** intermediate entries follow the interpolation, pixels retain their indices, and their appearance reflects the edited table

#### Scenario: Load and save an ACT table

- **WHEN** a user saves a table with an active count and transparent index as ACT and loads it into an indexed image whose used indices fit the table
- **THEN** the entries, active count and transparent index are restored and the image retains its indices

#### Scenario: Reject an ACT table that omits a used index

- **WHEN** a user loads an ACT table shorter than the highest used index plus one
- **THEN** the load is rejected and the dialog table, document and history remain unchanged

#### Scenario: Accept an unchanged table

- **WHEN** a user accepts Color Table without changing it
- **THEN** the table, pixels and history remain unchanged

### Requirement: Indexed metadata survives native persistence

Native PSD and PSB output SHALL preserve the active palette count and transparent
index of an editable indexed document across save and reopen, along with its
indices and resulting alpha.

#### Scenario: Indexed metadata round trips through PSD and PSB

- **WHEN** an indexed image with a short active table and a nonzero transparent index is saved and reopened as PSD and PSB
- **THEN** each reopened document retains the active count, transparent index and pixel indices, and its pixels have the corresponding alpha
