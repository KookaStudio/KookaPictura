# Spec Delta

## ADDED Requirements

### Requirement: Effect passes build for the composite region

A non-banded sub-region composite SHALL build each enabled effect pass over
the requested region grown by that effect's build reach, and the resulting pass
SHALL equal the same rectangle of a pass built over the whole layer, byte for
byte. The build reach SHALL be the effect's summed stage support plus its sample
offset where the effect reads a shifted sample: drop shadow and inner shadow grow
by `reach + |ox| + |oy|`, satin already includes its offset in its pad. The
build SHALL sample the layer content, the raster mask, and the vector mask only
over that grown region, or over one shared per-layer region that covers every
enabled family's reach, and SHALL clip every pass output to the requested
region. A gradient or pattern overlay's source buffer SHALL likewise be
generated only over the region its pass covers.

A full-canvas composite SHALL keep reusing the content-keyed cached pass. A
non-banded sub-region pass SHALL be built transiently and SHALL NOT be retained
in the shared cache, so a stroke cannot evict the cached sources of other
layers. A banded composite MAY build the whole-layer pass and replay it into
each band, keeping the cached full pass. This refines the region-composite
contract: the pass is built for the region rather than sliced from a
full-document pass, and the region output still equals the full composite's
slice.

#### Scenario: A region pass equals the full pass

- **WHEN** a document with a drop shadow, outer glow, inner shadow, inner glow,
  stroke, satin, bevel, and color, gradient, and pattern overlay is composited
  over a sub-region and whole
- **THEN** every pixel of the region composite equals the same pixel of the full
  composite, for every effect family

#### Scenario: A full composite still reuses its cached pass

- **WHEN** an unchanged styled document is composited whole twice
- **THEN** the second composite reports a pass-cache hit and rebuilds no effect
  pass

#### Scenario: A stroke present rebuilds only the dab and its reach

- **WHEN** a dab is painted on a canvas-sized layer carrying a layer style and
  the dab's present region is composited
- **THEN** each pass is built over the present region grown by its reach, not
  over the layer, and the composited pixels equal the same slice of a full
  composite

#### Scenario: An overlay source is bounded by the region

- **WHEN** a canvas-sized layer carries a gradient or pattern overlay and a
  sub-region is composited
- **THEN** the overlay's source buffer covers only the pass's region and the
  result equals the full composite's slice

#### Scenario: A banded sub-region reuses one pass

- **WHEN** a tall sub-region is composited in bands
- **THEN** one whole-layer pass is built and replayed into every band, and the
  output equals the whole composite

#### Scenario: A sub-region pass is not retained

- **WHEN** a non-banded sub-region composite runs
- **THEN** the shared cache does not gain a pass entry for that region

## MODIFIED Requirements

### Requirement: Content changes invalidate cached sources

A cached source or effect pass SHALL be recomputed when any input that can change
its pixels changes, including the layer rect, the embedded payload, the smart
filter stack, the type model, the font registry or missing-glyph setting, the
layer mask or vector mask and its refinements, the effect descriptor, and the
document size or colour mode. A pass whose build reads the document pattern
library SHALL NOT be cached, so a pattern edit cannot serve a stale pass.

#### Scenario: Replacing smart-object contents invalidates

- **WHEN** a smart object's embedded payload is replaced and the document is
  composited again
- **THEN** the new payload is decoded and the output matches a purge-then-composite

#### Scenario: Editing text invalidates

- **WHEN** a type layer's text, style, box, direction, or the available fonts
  change and the document is composited again
- **THEN** the raster is rebuilt and the output matches a purge-then-composite

#### Scenario: Editing an effect invalidates

- **WHEN** a layer-style parameter, the effect master switch, the layer mask, or
  the layer rect changes and the document is composited again
- **THEN** the effect passes are rebuilt and the output matches a
  purge-then-composite

#### Scenario: A painted layer invalidates its region pass

- **WHEN** a dab writes a layer whose effects are on and a sub-region is
  composited
- **THEN** the pass for that region is rebuilt from the current content and
  equals a pass built from scratch

### Requirement: Repeated composites reuse per-layer sources

The renderer SHALL reuse an embedded smart-object source, smart-filter chain
output, type raster, decoded effects, and effect passes across composites while
the layer's content identity and the document state that affects it are
unchanged. Effect-pass reuse SHALL apply to full-canvas composites; a non-banded
sub-region composite MAY build a transient pass instead of reusing the cached
one.

#### Scenario: A second composite hits

- **WHEN** an unchanged document containing a proxy-less smart object, a
  smart-filtered layer, a proxy-less type layer, and a styled layer is composited
  twice
- **THEN** the second composite reports cache hits for each of those layers and
  does not repeat their decode, filter, shape, or effect build work

#### Scenario: A hidden layer costs nothing

- **WHEN** a smart-object or type layer is not visible
- **THEN** its source is not decoded or cached for that composite and the
  backdrop is unchanged

### Requirement: The cache demonstrates a measurable speedup

An ignored benchmark SHALL measure a 4000×4000 document containing a
smart-filtered layer, a text layer with a drop shadow, and a canvas-sized styled
layer, comparing a 512×120 region composite (short enough to use the region path,
not banding) and repeated full composites against a purged baseline, and
comparing a dab-sized region composite on the canvas-sized styled layer with a
full composite.

#### Scenario: A warm region composite is at least ten times faster

- **WHEN** the benchmark runs a 512×120 region composite near the smart-filtered
  and styled layers with a warm cache
- **THEN** it is at least ten times faster than the same composite with a purged
  cache, and repeated full composites report cache hits

#### Scenario: A canvas-sized styled present beats a full composite

- **WHEN** a dab-sized region composite on a canvas-sized styled layer is timed
  against a full composite
- **THEN** the region composite is at least ten times faster and equals the full
  composite's slice
