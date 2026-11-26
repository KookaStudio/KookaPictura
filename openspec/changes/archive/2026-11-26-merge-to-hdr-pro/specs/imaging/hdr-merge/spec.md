## ADDED Requirements

### Requirement: Resolve an exposure bracket

File > Automate > Merge To HDR Pro SHALL accept browsed files, a folder, and clean saved open documents without modifying any input. The picker SHALL require at least three distinct readable, equal-sized sources. Inputs above 8 bits per channel SHALL be refused visibly before decoding. Complete JPEG and TIFF exposure metadata SHALL populate EV100 values; missing values SHALL use deterministic relative image-based estimates anchored to available metadata. Invalid or unrepresentable exposure ratios SHALL be rejected.

#### Scenario: Reject two sources

- **WHEN** only two sources are selected in the picker
- **THEN** the picker SHALL prevent proceeding and explain that at least three sources are required

#### Scenario: Reject a high-depth source

- **WHEN** a source has more than 8 bits per channel
- **THEN** the command SHALL explain the precision limit and publish no document

#### Scenario: Estimate missing exposure values

- **WHEN** three matching sources include rows without complete exposure metadata
- **THEN** the command SHALL estimate missing EV100 values from the images and open the preview without another exposure-entry dialog
- **AND** cancelling SHALL leave all input files and documents untouched

#### Scenario: Add clean saved open documents

- **WHEN** open documents are offered as sources
- **THEN** only clean saved documents SHALL be eligible and inclusion SHALL NOT save or change them

### Requirement: Build a floating-point radiance image

The merge SHALL combine encoded samples in an exposure-normalized linear domain, normalized to the median bracket exposure. It SHALL estimate a monotonic per-channel inverse response as the sRGB inverse transfer followed by a fitted positive exponent, use exponent 1 when evidence is insufficient, and support reusing a validated saved Pictura response curve. Optional translation alignment SHALL compensate supported displacement and SHALL fail visibly when registration cannot be established. Optional ghost removal SHALL use a selectable and visibly indicated base exposure where usable samples disagree. Out-of-frame and fully transparent samples SHALL be excluded; saturation and alpha fallback SHALL be deterministic as specified in the design.

#### Scenario: Recover scene radiance

- **WHEN** a synthetic three-exposure bracket represents known linear scene radiances including a highlight clipped in brighter exposures
- **THEN** reconstructed radiances SHALL match the analytical median-exposure-normalized values within a stated numerical tolerance and preserve the highlight above nominal white

#### Scenario: Reuse a response curve

- **WHEN** the same bracket is merged again using its saved Pictura response curve
- **THEN** the same floating-point radiance image SHALL result
- **AND** malformed or unsupported curves SHALL be rejected without publishing a document

#### Scenario: Recover a translated exposure

- **WHEN** a textured source is shifted within the supported translation search and alignment is requested
- **THEN** the merge SHALL use registered samples and exclude out-of-frame samples

#### Scenario: Reject failed alignment

- **WHEN** registration lacks sufficient texture or confidence, or displacement exceeds the supported search
- **THEN** the merge SHALL report failure and publish no document

#### Scenario: Resolve conflicting moving pixels

- **WHEN** the user selects a base and well-exposed samples disagree because an object moved with ghost removal enabled
- **THEN** the base SHALL be visibly indicated and its usable sample SHALL supply the affected pixel

#### Scenario: Fall back for saturated and transparent samples

- **WHEN** all usable samples at a pixel have zero fusion weight or no source has nonzero alpha
- **THEN** the deterministic saturation and alpha fallback in the design SHALL produce the pixel

### Requirement: Preview and publish a merged document

The command SHALL create a temporary 32-bit document with a bottom placeholder named Merge to HDR Pro and a layer for each selected source. It SHALL show the selected sources, estimated EV labels and adjustable lighting controls in a preview. Preview brightness SHALL NOT change merged pixels. The command SHALL support Pictura preset and response-curve save and load actions. Acceptance SHALL replace the temporary stack with one locked Background containing float samples. The job SHALL check and reserve its decoded photo and working memory against the existing budget before allocation. Overflow, budget failure, cancellation, or any other failure SHALL release temporary resources, leave all input files and existing documents untouched, and remove the temporary document. Persisting preview white point on PSD/PSB/TIFF reopen remains a #558 parity gap.

#### Scenario: Adjust preview brightness

- **WHEN** preview brightness is adjusted
- **THEN** the display SHALL change while merged floating-point samples remain byte-identical

#### Scenario: Publish or cancel the merge

- **WHEN** the preview is accepted at 32 bits per channel
- **THEN** exactly one untitled 32-bit document SHALL open with the merged samples
- **AND** cancellation at source selection or preview SHALL remove the temporary document and leave inputs untouched

#### Scenario: Inspect temporary layers and final background

- **WHEN** the preview is open after selecting three sources
- **THEN** the temporary 32-bit document SHALL contain the placeholder and three source layers
- **AND** accepting SHALL replace them with one locked Background layer

#### Scenario: Reject an over-budget job

- **WHEN** checked memory estimation overflows or the photo and working memory reservation is refused
- **THEN** the command SHALL report failure, release its resources, and leave inputs and document count unchanged

### Requirement: Refine an HDR merge in a full-screen preview

The HDR preview SHALL use a full-screen dialog with the image on the left, controls and confirmation on the right, and a horizontally scrollable strip of source thumbnails below. Advanced lighting controls and the editable luminance curve SHALL have separate tabs.

#### Scenario: Toggle HDR source inclusion

- **WHEN** at least three source files have been accepted and the preview opens
- **THEN** all sources SHALL initially be included and represented by checked thumbnails
- **AND** excluding a source SHALL hide its temporary layer and recompute the preview from the remaining sources
- **AND** with two included sources the checked boxes SHALL be disabled while excluded sources remain available to restore
- **AND** accepting SHALL merge the included sources only and canceling SHALL discard the temporary stack

#### Scenario: Edit the HDR tone curve

- **WHEN** the Curve tab opens
- **THEN** the graph SHALL show the input luminance histogram and start at identity
- **AND** users SHALL be able to insert and move points, enter the selected point's input and output percentages, choose corner interpolation, and reset the curve
- **AND** the curve SHALL map native float luminance continuously through display white and preserve highlight headroom
- **AND** loading a legacy scalar-curve preset SHALL reproduce its prior tone adjustment and Reset SHALL clear it
