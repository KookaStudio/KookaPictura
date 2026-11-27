# ui/blend-if-controls Specification

## Purpose
Allow users to edit and preview supported layer tonal blending ranges from Blending Options.

## Requirements

### Requirement: Edit layer blending ranges
The Blending Options page SHALL expose Gray ranges in RGB and grayscale documents and Red, Green and Blue ranges in RGB documents, for This Layer and Underlying Layer. Other document modes and pass-through groups SHALL show unavailable controls, disable replacement, and preserve existing ranges. Each editable range SHALL offer black and white handles with split halves. Editing one range SHALL preserve all other typed ranges and channel groups.

#### Scenario: Edit source and backdrop [bif_ranges]
- **WHEN** the user edits source and backdrop handles on Gray and an RGB channel
- **THEN** the canvas previews each change and the other ranges remain unchanged

#### Scenario: Unsupported document mode [bif_modes]
- **WHEN** Blending Options opens for a CMYK, Lab, or other unsupported mode
- **THEN** the controls and replacement action are unavailable and existing raw and typed ranges remain unchanged through save

#### Scenario: Pass-through group [bif_group]
- **WHEN** Blending Options opens for a pass-through group
- **THEN** editing and replacement are unavailable and the existing ranges remain unchanged

#### Scenario: Split a handle [bif_split]
- **WHEN** the user splits and moves black or white handles for source or backdrop
- **THEN** the interval follows the ordered linear factors and inclusive equal-endpoint steps defined in the design, defaults retain both extremes, and the range survives reopening

#### Scenario: Commit and cancel [bif_history]
- **WHEN** the user accepts or cancels range edits
- **THEN** acceptance creates one undoable change and cancellation restores the opening typed and raw state

#### Scenario: Save ranges [bif_save]
- **WHEN** an edited layer is saved to PSD and reopened
- **THEN** its source, backdrop, and channel ranges are retained

#### Scenario: Cached preview updates [bif_cache]
- **WHEN** a lower layer's range is edited repeatedly while a cached composite prefix exists
- **THEN** every preview and committed composite reflects the latest range

### Requirement: Preserve opaque blending ranges
An unparseable nonempty blending-ranges block SHALL disable editing until the user explicitly chooses to replace it after being told replacement discards the opaque data. Opening, declining replacement, cancelling the dialog, or saving without replacement SHALL preserve the block byte-for-byte. Replacement SHALL start at default ranges and be undoable.

#### Scenario: Opaque ranges are preserved or deliberately replaced [bif_opaque]
- **WHEN** a layer has an unparseable nonempty block
- **THEN** edits are disabled until explicit replacement; declining or cancelling retains the exact block, while accepting replacement creates one undoable change
