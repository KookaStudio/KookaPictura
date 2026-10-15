# smart-object-layer-actions Specification (delta)

## REMOVED Requirements

### Requirement: Convert to Smart Object eligibility and refusal

**Reason**: The Background layer is no longer an ineligible target. The operation
now converts it (unlocking it first), so this requirement's scope is replaced by
`Convert to Smart Object eligibility`. The OpenSpec validator forbids renaming a
scenario under `MODIFIED`, so the requirement is retired and re-added rather than
edited in place.

**Migration**: Use `Convert to Smart Object eligibility`, which keeps every other
guard (8-bit document, non-group, no adjustment data, no existing smart object,
positive `rect`) and adds Background eligibility.

## ADDED Requirements

### Requirement: Convert to Smart Object eligibility

The engine operation `pictura-render::convert_to_smart_object(doc, path)` SHALL
convert the layer at `path` only when it is a single raster pixel layer in an
8-bit document: the path resolves, the document is 8-bit, the layer is not a
group, has no adjustment data, has no existing smart object, and has a `rect`
with positive width and height. The Background layer SHALL be eligible. On any
other target the operation SHALL return `false` and SHALL NOT mutate the
document.

#### Scenario: A raster pixel layer converts

- **WHEN** `convert_to_smart_object` is called on the path of a raster pixel layer that has no smart object
- **THEN** it returns `true` and the layer carries an embedded smart object

#### Scenario: A Background layer converts

- **WHEN** `convert_to_smart_object` is called on the Background layer
- **THEN** it returns `true`, the layer carries an embedded smart object, and it is no longer the Background

#### Scenario: A group is refused

- **WHEN** the target path resolves to a group layer
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An adjustment layer is refused

- **WHEN** the target layer has adjustment data
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An existing smart object is refused

- **WHEN** the target layer already has a smart object
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A zero-size layer is refused

- **WHEN** the target layer's `rect` has zero width or zero height
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: A non-8-bit document is refused

- **WHEN** the document's bit depth is not 8-bit
- **THEN** the operation returns `false` and the document is unchanged

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the operation returns `false` and the document is unchanged

### Requirement: A Background is unlocked before conversion

When the target of `convert_to_smart_object` is the Background layer, the
operation SHALL first make it an ordinary layer — clear the Background flag,
release its locks, add an opaque alpha channel when it has none, and rename it to
the document's next free `Layer N` — and SHALL then embed its raster as an
embedded smart object. The unlock SHALL be part of the same operation, SHALL NOT
record a separate state, and SHALL NOT change the composite.

#### Scenario: The converted layer is an ordinary layer

- **WHEN** the Background layer is converted
- **THEN** its Background flag is cleared and its locks are released

#### Scenario: A channel-less Background gains an opaque alpha channel

- **WHEN** the Background layer has no alpha channel and is converted
- **THEN** the layer carries an opaque alpha channel

#### Scenario: The composite is unchanged

- **WHEN** the Background layer is converted
- **THEN** sampling the composite before and after returns the same pixels

#### Scenario: The name becomes the next free Layer N

- **WHEN** the Background layer is converted
- **THEN** the layer is renamed to the document's next free `Layer N`

## MODIFIED Requirements

### Requirement: The app command records exactly one undo state

The application SHALL expose the command `Layer > Smart Objects > Convert to
Smart Object` with the stable id `LayerSmartObjectConvertTo`. The command SHALL
be enabled only when the current layer is convertible, SHALL invoke the engine
operation on success, clear the link sets, recomposite, and record exactly one
history state labelled `"Convert to Smart Object"`. A refusal SHALL record no
history state.

#### Scenario: Success records one state

- **WHEN** the command runs on a convertible current layer
- **THEN** exactly one history state labelled `"Convert to Smart Object"` is added and the layer reports a smart object

#### Scenario: A Background success records one state

- **WHEN** the command runs on the Background layer
- **THEN** exactly one history state labelled `"Convert to Smart Object"` is added, the layer is no longer the Background, and it reports a smart object

#### Scenario: Refusal records nothing

- **WHEN** the command runs on a group, an adjustment layer, or an existing smart object
- **THEN** the history count is unchanged and the document is unchanged

#### Scenario: Enabled only when convertible

- **WHEN** the current layer is not convertible
- **THEN** the command is disabled and its availability predicate mutates nothing

### Requirement: The raster proxy and rendering are unchanged

The operation SHALL keep the layer's existing pixel channels as a raster proxy
and, except when the target is the Background, SHALL NOT change the layer's name,
rect, blend, opacity, fill, mask, or position. When the target is the Background,
the operation SHALL apply the unlock described by `A Background is unlocked
before conversion` — renaming the layer and adding an opaque alpha channel — and
SHALL otherwise leave the layer's rect, blend, opacity, fill, mask, and position
unchanged. Because a layer with a proxy renders from the proxy, the document
composite after conversion SHALL equal the composite before conversion.

#### Scenario: Pixel channels survive

- **WHEN** a raster layer that is not the Background and has color channels is converted
- **THEN** the layer's channels are byte-for-byte the same as before

#### Scenario: The composite is unchanged

- **WHEN** the service composite is compared before and after a successful conversion
- **THEN** the two composites are equal

#### Scenario: Rejected destinations are untouched beyond the refusal

- **WHEN** conversion is refused
- **THEN** the layer's channels, rect, name, and smart-object state are unchanged
