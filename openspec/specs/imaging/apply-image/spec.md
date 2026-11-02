# imaging/apply-image Specification

## Purpose
Image > Apply Image: blend a layer and channel of an open document into the
active layer or layer mask, through an optional mask, at the document's depth.

## Requirements

### Requirement: Apply Image dialog

`Image ▸ Apply Image…` SHALL open a modal dialog with Source, Layer, Channel, Invert, a read-only Target, Blending, Opacity, Preserve Transparency, an optional Mask group, and Preview. Blending SHALL default to Multiply and Opacity to 100 %. The command SHALL be disabled without a filterable target, on a Smart Object, and on Indexed or Bitmap documents.

#### Scenario: Sources must match the target's dimensions

- **WHEN** another open document has different pixel dimensions from the active document
- **THEN** it is absent from the Source list, and the active document is listed first

#### Scenario: Composite is hidden across colour modes

- **WHEN** the source's colour mode differs from the target's
- **THEN** the Channel list offers single channels and no composite entry

#### Scenario: Scale and Offset follow the mode

- **WHEN** Blending is Add or Subtract
- **THEN** Scale and Offset are shown, and they are hidden for every other mode

#### Scenario: The mode list

- **WHEN** the Blending list is read
- **THEN** it holds the layer modes without Dissolve, Multiply is selected, and Subtract appears once

### Requirement: Apply Image blending

Apply Image SHALL blend the chosen source into the target per pixel, with coverage from source alpha, Opacity, the mask, and the selection. Add SHALL compute `(A + B) / Scale + Offset` and Subtract `(A - B) / Scale + Offset`, where A is the target, B the source, and Offset is `offset / 255` in the unit domain. Invert SHALL use the complement of the source.

#### Scenario: Add with Scale 2 averages

- **WHEN** Add runs with Scale 2 and Offset 0
- **THEN** each result is the average of target and source within 1/255

#### Scenario: Subtract honours Scale and Offset

- **WHEN** Subtract runs
- **THEN** a higher Scale reduces the result and a positive Offset raises it

#### Scenario: Layer modes use the shared blend

- **WHEN** a listed layer mode runs at full coverage on opaque pixels
- **THEN** the result equals the compositor's blend function for that mode

#### Scenario: A single channel feeds every target channel

- **WHEN** the source channel is Red
- **THEN** the blend sees the red value on all three target channels

#### Scenario: Opacity and Invert

- **WHEN** Opacity is 50 % with Normal, or Invert is set
- **THEN** the result is halfway to the source, or uses the source's complement

#### Scenario: Preserve Transparency

- **WHEN** Preserve Transparency is on and the layer has clear pixels
- **THEN** those pixels and all alpha are unchanged

#### Scenario: Transparent target takes the source

- **WHEN** Preserve Transparency is off and a clear pixel receives an opaque source
- **THEN** the pixel becomes opaque with the source colour

### Requirement: Apply Image mask

The Mask group SHALL accept a chosen image, layer, and channel, or Transparency, or the active selection, with Invert. The mask's value SHALL scale coverage.

#### Scenario: Selection as mask

- **WHEN** the mask channel is Selection
- **THEN** the blend applies only where the selection covers, with coverage as strength

#### Scenario: Inverted channel mask

- **WHEN** a channel mask is inverted
- **THEN** the blend applies where the channel is dark

#### Scenario: Active selection confines the result

- **WHEN** a selection is active and no Mask is set
- **THEN** only selected pixels change

### Requirement: Apply Image preview and history

Previews SHALL NOT compound, Cancel SHALL restore the target exactly, and OK SHALL commit one "Apply Image" history state that undoes. A preview of a section SHALL equal the same area of a full apply.

#### Scenario: Cancel restores

- **WHEN** the dialog previews and is cancelled
- **THEN** the target pixels are unchanged and no history state is added

#### Scenario: OK commits one state

- **WHEN** OK is pressed
- **THEN** one "Apply Image" state is added and undo restores the pixels

#### Scenario: Merged from the active document does not compound

- **WHEN** the source is the active document's Merged and a control changes twice
- **THEN** the preview equals one application to the original pixels

#### Scenario: Section matches a full apply

- **WHEN** the kernel runs over a section
- **THEN** that section equals the full apply and nothing outside it changes

### Requirement: Apply Image targets

The active layer mask, when it is the edit target, SHALL receive the blend as a gray target. A 16- or 32-bit layer whose native store owns its colour planes SHALL be blended in the store at native depth, with the 8-bit channels re-narrowed; a 32-bit target SHALL NOT clamp Add or Subtract results.

#### Scenario: Mask target

- **WHEN** a layer mask is the edit target and OK is pressed
- **THEN** the mask coverage changes and the layer's pixels do not

#### Scenario: 16-bit keeps precision

- **WHEN** a 16-bit layer takes a blend
- **THEN** the store holds values finer than 8-bit steps and the channels equal its narrowing

#### Scenario: 32-bit does not clamp

- **WHEN** Add on a 32-bit layer exceeds 1.0
- **THEN** the store keeps the value above 1.0

#### Scenario: Cancel restores a native store

- **WHEN** a native-depth preview is cancelled
- **THEN** the store is bit-identical to the original
