# compositing/layer-source-cache Specification

## Purpose
Caches the expensive per-layer render sources (embedded smart objects, smart
filter chains, type rasters, decoded layer effects, and effect passes) by content
identity, so repeated and partial composites reuse them while producing exactly
the same pixels as an uncached render.

## Requirements

### Requirement: Cached rendering is byte-identical to uncached rendering

The renderer SHALL produce identical pixels whether a source or effect pass is
built fresh or reused from the cache. A composite run after a cache purge and the
same composite run against a warm cache SHALL be byte-for-byte equal for every
document the oracle suites cover. The smart-filter geometry fix SHALL change only
region output: a full composite SHALL remain byte-identical to the previous full
composite.

#### Scenario: Cold and warm composites agree

- **WHEN** a document with smart objects, smart filters, type layers, masks, and
  layer effects is composited once with a purged cache and once again warm
- **THEN** the two outputs are byte-identical

#### Scenario: Existing oracle suites still pass

- **WHEN** the change is applied
- **THEN** the existing composite, layer-effect, smart-object, smart-filter, and
  knockout oracle suites pass without baseline changes

### Requirement: Repeated composites reuse per-layer sources

The renderer SHALL reuse an embedded smart-object source, smart-filter chain
output, type raster, decoded effects, and effect passes across composites while
the layer's content identity and the document state that affects it are
unchanged.

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

### Requirement: Region composites draw cached effects

A region composite SHALL equal the same rectangle sliced from a full composite
for every layer whose effects have a bounded reach, including smart-filtered and
proxy-less layers. The renderer SHALL NOT substitute a full-document composite
and slice when the requested region intersects a bounded effect.

#### Scenario: A styled layer's region matches the full slice

- **WHEN** a region intersecting a drop shadow, outer or inner glow, inner
  shadow, stroke, satin, bevel, or overlay is composited
- **THEN** every pixel equals the same pixel of the full composite

#### Scenario: A smart-filtered region matches the full slice

- **WHEN** a region of a layer carrying a geometric smart filter (Lens
  Correction) is composited
- **THEN** every pixel equals the same pixel of the full composite

#### Scenario: No bounded effect forces a full composite

- **WHEN** the requested region intersects a layer effect with a bounded reach
- **THEN** only the region is composited and the full document is not

### Requirement: A layer's effects descriptor decodes once per composite

For one layer in one composite, the renderer SHALL resolve its effect descriptor
at most once, regardless of how many effect families the descriptor enables or
how many times the layer's effect reaches are inspected.

#### Scenario: One decode for a multi-effect layer

- **WHEN** a layer enables a drop shadow, inner shadow, stroke, and bevel
- **THEN** its descriptor is decoded once for that composite and the same decoded
  value feeds every effect pass and the reach calculation

### Requirement: Effect passes replay without changing output

The renderer SHALL build each enabled effect into document-space passes and
replay them into a canvas rectangle through the same per-pixel blend used by an
uncached render. Pass replay SHALL preserve the effect's blend mode, colour
(constant or per-pixel), alpha, noise, and the layer's effect opacity scaling.

#### Scenario: A constant-colour effect replays exactly

- **WHEN** a drop shadow or glow pass is replayed on a canvas
- **THEN** every touched pixel equals the uncached result

#### Scenario: A per-pixel-colour effect replays exactly

- **WHEN** a gradient overlay, pattern overlay, or stroke pass is replayed
- **THEN** every touched pixel equals the uncached result

### Requirement: Banded compositing covers cached source layers

When a region is tall enough to composite in bands, the banded result SHALL equal
the whole-canvas composite for stacks containing smart objects, type layers,
smart filters, and layer effects.

#### Scenario: Banded equals whole with object layers

- **WHEN** a tall document containing a smart-filtered layer, a proxy-less type
  layer, and a styled layer is composited in bands and whole
- **THEN** the two outputs are byte-identical

### Requirement: Layer Style preview bounds its recomposite

A Layer Style preview update SHALL recomposite only the union of the layer paint
bounds before and after the change, clipped to the canvas, rather than the whole
document. A pixel, smart-object, type, or fill layer's paint bounds SHALL be its
rect unioned with its effect reach. A group's paint bounds SHALL be the union of
its descendants' paint bounds. An adjustment layer's paint bounds SHALL be the
document canvas, or its mask bounds when the mask is bounded. A missing path
SHALL have no paint bounds.

#### Scenario: A slider tick recomposites the paint bounds union

- **WHEN** a Layer Style value changes an effect size, distance, spread, opacity,
  colour, or visibility, or a group's or adjustment layer's blending value
- **THEN** the damaged rectangle is the union of the before and after paint
  bounds and is smaller than the document when those bounds are smaller

#### Scenario: A non-pixel style change does nothing

- **WHEN** a requested style value is refused or leaves the document unchanged
- **THEN** no recomposite is requested

### Requirement: Caches are bounded, purgeable, and reported

The renderer SHALL keep cached sources and passes in one process-wide cache with
a default 256 MiB byte budget, evicting least-recently-used entries when the
budget is exceeded, SHALL build an artifact larger than the budget but not store
it, SHALL expose an entry point that purges the cache, and SHALL report cache
hits, misses, resident bytes, and effect decodes through the metrics counters.

#### Scenario: The budget evicts

- **WHEN** cached artifacts exceed the configured byte budget
- **THEN** older entries are evicted and later requests rebuild them correctly

#### Scenario: Purge forces rebuilds

- **WHEN** the cache is purged
- **THEN** the next composite misses and produces the same pixels as before

#### Scenario: Counters are visible

- **WHEN** the metrics report is read after composites
- **THEN** it includes source-cache hits, misses, resident bytes, and effect
  decodes

### Requirement: The cache demonstrates a measurable speedup

An ignored benchmark SHALL measure a 4000×4000 document containing a
smart-filtered layer and a text layer with a drop shadow, comparing a 512×120
region composite (short enough to use the region path, not banding) and repeated
full composites against a purged baseline.

#### Scenario: A warm region composite is at least ten times faster

- **WHEN** the benchmark runs a 512×120 region composite near the smart-filtered
  and styled layers with a warm cache
- **THEN** it is at least ten times faster than the same composite with a purged
  cache, and repeated full composites report cache hits
