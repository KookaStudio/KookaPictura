## ADDED Requirements

### Requirement: Curve points and display options

Both Curves surfaces SHALL expose exact numeric input/output and movable black and white input endpoints. They SHALL support simple and detailed grids, a histogram, a baseline, component overlays on a composite graph, and alignment guides. Changing display settings SHALL preserve the adjustment. CMYK SHALL retain a composite and four independent component curves. Non-RGB edits SHALL transform the RGB-backed input into the document mode, apply the curves there, and transform back. Identity SHALL bypass conversion. Non-RGB Curves SHALL use CPU rendering rather than the GPU independent-channel LUT path.

#### Scenario: Edit endpoints and graph display [CURVE-DISPLAY]

- **WHEN** a user drags the white endpoint to 243 and changes grid or overlay settings
- **THEN** inputs at and above 243 map to maximum output and display changes leave the curve data intact

#### Scenario: View mode-appropriate channels and values [CURVE-MODES]

- **WHEN** a user opens Curves on Gray, CMYK or Lab documents
- **THEN** channel choices and numeric labels correspond to the document mode, a component curve adjusts that component, and all CMYK component and composite curves survive simultaneous editing and document reopening

### Requirement: Freehand curves

Both Curves surfaces SHALL support freehand drawing and repeatable smoothing. Each completed stroke or smoothing action SHALL be one Properties history edit. The 256 samples SHALL survive a Pictura adjustment-layer document round trip and edits to other channels. Unedited source adjustment content SHALL be encoded byte-for-byte. Edited content SHALL preserve unaffected channels and unrelated source content. A malformed or unsupported private trailer SHALL leave the sparse fallback in effect without partially applying dense tables.

#### Scenario: Draw, smooth and reopen [CURVE-PENCIL]

- **WHEN** a user draws across several input levels, smooths twice, edits a different channel and saves the layer
- **THEN** intermediate inputs are continuous across each drawn segment, repeated smoothing changes the table, and reopening preserves the final table and sibling edit

### Requirement: Image-based Curves editing

Both Curves surfaces SHALL support targeted editing and black, gray and white sampling when appropriate to the channel mode. A targeted drag SHALL use the press sample's input tone and move it vertically. Gray sampling in RGB SHALL map the sampled color to neutral. A dialog cancellation SHALL discard the changes and return control to the canvas. A Properties target change SHALL cancel an active pick. Each completed Properties gesture or sample SHALL create one undoable edit.

#### Scenario: Target a tone and sample endpoints [CURVE-SAMPLE]

- **WHEN** a user targets a tone and drags, or samples with a black or white endpoint pipette
- **THEN** the targeted point keeps the press tone as its input while its output moves, or the sampled tone maps to the respective endpoint, and closing or changing the target releases the canvas

#### Scenario: Neutralize a cast [CURVE-GRAY]

- **WHEN** a user picks a colored pixel with the RGB gray point tool
- **THEN** the sample maps to a neutral gray in a single edit

### Requirement: Automatic Curves correction

Both Curves surfaces SHALL provide Auto for the selected RGB or Gray channel using the existing fixed histogram stretch, discarding 0.1% at each tail. Auto SHALL be disabled for CMYK and Lab. Empty histograms and ranges narrower than two levels SHALL leave the adjustment and history unchanged. No Auto Options are introduced.

#### Scenario: Apply Auto [CURVE-AUTO]

- **WHEN** a user chooses Auto on an RGB or Gray channel with a valid narrower-than-full tonal range
- **THEN** the selected curve becomes a two-point stretch using the shared bounds and the operation uses the existing preview and commit path
