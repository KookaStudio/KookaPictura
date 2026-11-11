# compositing/region-composite Specification

## Purpose
The composite of any rectangle at any view-pyramid level, byte-identical to
the same crop of a whole composite and of a whole pyramid rebuild, and the
readers (save, flatten, Sample All Layers) that stream it instead of holding a
canvas-sized composite.

## Requirements

### Requirement: Composite a region at a pyramid level

The renderer SHALL provide `composite_region(doc, rect, level)`, which returns
the composite of the document over `rect` at pyramid `level`, without compositing
or storing the rest of the document. At level 0 the result SHALL be straight-alpha
working-space RGBA and SHALL be byte-identical to the same rectangle cropped from
the whole-document composite. At a level k above 0 the rectangle SHALL be given
in level-k pixels and the result SHALL be premultiplied RGBA byte-identical to the
same rectangle cropped from level k of a view pyramid rebuilt from the whole
composite, including the folding of odd rows and columns at the level's edges.
Pixels of `rect` outside the level SHALL be transparent.

#### Scenario: A level-0 region equals the full crop [rc_level0]

- **WHEN** a region is composited at level 0 for seeded documents covering every blend mode the oracle covers
- **THEN** it equals the same rectangle cropped from the whole composite, byte for byte

#### Scenario: A level-k region equals the pyramid crop [rc_levels]

- **WHEN** regions are composited at levels 1 through 3, including rectangles that touch odd-sized right and bottom edges
- **THEN** each equals the same rectangle cropped from that level of a pyramid rebuilt from the whole composite, byte for byte

#### Scenario: A region past the level is transparent [rc_outside]

- **WHEN** a requested rectangle extends past the level's bounds
- **THEN** the part outside is fully transparent and the part inside matches the crop

### Requirement: Large regions stream in bands

A region whose level-0 footprint exceeds one band SHALL be composited in bands of
rows aligned to `2^level`, each band halved into the output before the next is
composited, so the peak working memory is bounded by one level-0 band and the
output rather than by the region's level-0 footprint. The streamed result SHALL
equal the unbanded result byte for byte.

#### Scenario: A streamed region equals the unbanded one [rc_banded]

- **WHEN** a whole-document region at a level above 0 is composited with a band smaller than its footprint
- **THEN** the result equals the unbanded composite of the same region, and no buffer of the region's level-0 size is allocated

### Requirement: Readers stream the composite

Saving the PSD or PSB merged image of a layered RGB document, flattening, and the
Sample All Layers sources (bucket, magic wand, Grow, Similar, eyedropper, clone,
smudge, mixer and focus tools) SHALL read the composite as regions or streamed
bands and MUST NOT allocate one canvas-sized composite buffer. A document whose
composite is authoritative (no layers, a non-RGB mode, or a retained native-depth
merged composite) SHALL keep writing its stored composite unchanged.

#### Scenario: The streamed merged image is byte-identical [rc_save]

- **WHEN** layered RGB documents from the corpus are saved as PSD with the streamed merged image
- **THEN** the file bytes equal the file written from a whole composite

#### Scenario: Save allocates no whole composite [rc_save_peak]

- **WHEN** a large layered document is saved
- **THEN** no allocation of the document's full composite size is made by the merged-image path

#### Scenario: Flatten composites by band [rc_flatten]

- **WHEN** a layered document is flattened
- **THEN** the flattened layer equals the whole composite and no canvas-sized composite buffer is allocated beyond the new layer itself

#### Scenario: Sample All Layers reads regions [rc_sample_all]

- **WHEN** the magic wand, the paint bucket and the eyedropper sample all layers on a layered document
- **THEN** their results equal sampling the whole composite, and no whole composite is built
