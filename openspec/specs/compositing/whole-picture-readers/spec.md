# compositing/whole-picture-readers Specification

## Purpose
Defines how a reader that needs the whole picture reads it without a
canvas-sized composite: exact readers stream bands or tiles into their own
output, previews read a reduced pyramid level, conversions stream their RGB
source, and hidden-layer backdrops cover only what is shown.

## Requirements

### Requirement: Whole-output readers stream the composite into their output

A reader whose result covers the whole document SHALL composite the picture band
by band straight into its one output buffer, and MUST NOT allocate a
canvas-sized composite, sRGB copy or planar copy besides that output. This
applies to raster export, Save for Web, raster save formats, Duplicate (merged),
Load Channel Selection and the Color Range commit. A reader that only scans the
picture, such as Trim's coverage bounds or Match Color's source statistics,
SHALL keep no canvas-sized buffer at all. Copy Merged SHALL composite only the
selection's bounds.

#### Scenario: Export streams into one RGBA buffer [wp_export]

- **WHEN** a layered RGB document with a working profile is exported as PNG, saved for web, and saved as a raster format
- **THEN** each file is byte-identical to the one encoded from the whole composite
- **AND** the heap peak of the export path stays below the packed RGBA output plus half of one RGBA composite

#### Scenario: Copy Merged reads the selection bounds [wp_copy_merged]

- **WHEN** Copy Merged runs with a selection smaller than the document, and again with no selection
- **THEN** the clip equals the one cut from the whole composite
- **AND** with the selection, the composite read is no larger than the selection's bounds

#### Scenario: Trim and Duplicate (merged) stream [wp_image_ops]

- **WHEN** Trim runs on a document with transparent margins, and Duplicate (merged) runs on a layered document
- **THEN** Trim's bounds and the duplicate's pixels equal the whole-composite results
- **AND** Trim's heap peak holds no canvas-sized buffer

#### Scenario: Load Channel Selection streams [wp_channel_selection]

- **WHEN** a red, a green, a blue, and the composite channel row is loaded as a selection on a layered document
- **THEN** each selection equals the one derived from the whole composite

#### Scenario: Match Color statistics are accumulated by band [wp_match_color]

- **WHEN** the source statistics of a layered document are taken with and without a selection
- **THEN** the six statistics equal those taken over the whole composite bit for bit

### Requirement: Selection readers read the composite through tiles

Grow, Similar, the Magic Wand action, Refine Edge (refinement and
decontamination) and the Magnetic Lasso SHALL read the picture one tile at a
time, as Sample All Layers does, compositing only the tiles they touch. Their
results SHALL equal, sample for sample, the results computed over the whole
composite.

#### Scenario: Grow, Similar and the wand read tiles [wp_select_tiles]

- **WHEN** Grow, Similar and the wand action run on a seeded layered document at an odd size, once through tiles and once through the whole composite
- **THEN** each pair of selections is identical
- **AND** on a large document the heap peak stays below the selection plus half of one RGBA composite

#### Scenario: Refine Edge reads its edge band through tiles [wp_refine]

- **WHEN** Refine Edge with a radius, smoothing, feathering, contrast and decontamination is committed through tiles and through the whole composite
- **THEN** the refined masks and the decontaminated pixels are identical

#### Scenario: The Magnetic Lasso reads tiles [wp_magnetic]

- **WHEN** a magnetic wire is traced over a seeded layered document with the edge map built from tiles and from the whole composite
- **THEN** the two edge maps have the same peak and the traced paths are identical
- **AND** beginning the gesture on a large document allocates no canvas-sized composite

### Requirement: Previews read a reduced level

A reader that only feeds a preview SHALL read the composite at the coarsest
pyramid level that still covers the preview's size, and MUST NOT composite the
whole document at level 0 when the preview is smaller than the document. This
applies to the Color Range, Refine Edge and Replace Color dialog previews, the
control server's document capture and the Image Size preview. Such previews are
approximate. The result each dialog commits is computed by the exact readers
above.

#### Scenario: A preview reads the level it shows [wp_preview_level]

- **WHEN** a Color Range, a Refine Edge and a Replace Color preview of a large document is requested at a size of 256
- **THEN** the composite is read at the coarsest level at least that size, and the heap peak holds no canvas-sized composite
- **AND** on a document no larger than the preview, the preview equals today's level-0 preview

#### Scenario: A document capture reads a reduced level [wp_capture]

- **WHEN** the control server captures a large document with a maximum dimension, and the Image Size dialog opens on it
- **THEN** each image is built from a reduced level no larger than needed for that dimension, not from a whole level-0 image

### Requirement: Conversions stream their RGB source

The Grayscale, CMYK and Lab mode conversions and the bit-depth up-conversion
SHALL read a layered document's RGB picture band by band into the converted
document's stores. They MUST NOT first store a whole RGBA composite and then
copy it into flat whole-document vectors. The converted document SHALL equal the
one produced from the whole composite, with dithering enabled as well.

#### Scenario: A conversion streams its source [wp_conversion]

- **WHEN** a layered RGB document is converted to Grayscale, CMYK, Lab, and 16 bits
- **THEN** each converted document's composite, stores and layers equal those of the whole-composite conversion
- **AND** the Grayscale conversion's heap peak stays below its output plus half of one RGBA composite

### Requirement: Hidden-layer backdrops read regions or levels

A backdrop of the document with some layers hidden SHALL be composited over the
area that is shown. The Liquify backdrop SHALL composite its frame only. The
Vanishing Point backdrop SHALL be composited band by band into its one sRGB
output. The Move and transform backdrops SHALL be a patch over the hidden
layers' influence, the pixels whose value can depend on them, for a group, a
layer with effects and an adjustment layer as well. A whole-document backdrop
SHALL remain only where that influence cannot be bounded or the canvas does not
show the document as it stands.

#### Scenario: Liquify and Vanishing Point backdrops [wp_filter_backdrop]

- **WHEN** Liquify opens on a layer smaller than the document, and Vanishing Point opens on a layered document
- **THEN** each backdrop equals the same area of the whole composite with the layer hidden
- **AND** the Liquify composite read is no larger than its frame

#### Scenario: Move and transform backdrops cover the influence [wp_move_fallback]

- **WHEN** a group, a layer with a drop shadow, and a small adjustment layer's sibling is moved, and a layer with effects is transformed
- **THEN** the backdrop is a patch over the hidden layers' influence, at its document-space origin
- **AND** the patch equals the same area of the whole composite with those layers hidden
