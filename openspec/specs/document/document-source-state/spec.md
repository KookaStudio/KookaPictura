# document-source-state Specification

## Purpose

Keep document source metadata and composite storage internally consistent while providing one shared set of decisions to codec, render, history, and application consumers.

## Requirements

### Requirement: Document source and composite state have private invariants

A document SHALL own one encapsulated source value and one encapsulated composite
value. The former independently writable source mode, source depth, retained
planes, palette, original profile, working profile, composite buffer, and merged
image flag SHALL NOT remain public mutable fields. Construction and transitions
SHALL make contradictory combinations unrepresentable or reject them before
publishing the new state. Borrowing pixel samples for an edit SHALL NOT permit
changing the store's mode, depth, dimensions, or sample representation independently.

#### Scenario: Every supported state is constructible [dss_legal_states]

- **WHEN** callers construct working RGB/Grayscale states, recorded normalized modes, retained native samples, Bitmap packed samples, and Indexed samples with a palette, with a stored or absent composite as supported by existing read and conversion paths
- **THEN** construction succeeds and mode, depth, retained-depth, native-plane authority, and composite queries agree with the corresponding existing behavior

#### Scenario: Contradictory state is refused [dss_invalid_states]

- **WHEN** a caller supplies an Indexed representation without its palette, attaches a palette to a non-Indexed representation, or supplies retained samples whose representation contradicts their declared depth
- **THEN** the interface cannot express the combination or returns an error without changing the existing document

### Requirement: Source queries distinguish provenance from sample authority

Effective mode/depth, retained source depth, and native-plane authority SHALL have
one core implementation consumed by the render and application layers. A recorded
source depth alone SHALL NOT imply retained native samples. Retained-depth queries
SHALL include stores in descendant layers even when no document-level retained
store exists. Native-plane authority SHALL retain the existing source-depth,
converted-mode, layer rectangle, layer depth, and channel-id checks. A mismatch
SHALL keep the existing 8-bit fallback rather than invalidating an otherwise
loadable document.

#### Scenario: Recorded depth without samples remains provenance [dss_recorded_depth]

- **WHEN** a document records a 16-bit source depth but retains no high-depth samples in its composite or layers
- **THEN** retained-depth is false and its effective bit depth follows the existing 8-bit fallback

#### Scenario: Descendant layer samples retain depth [dss_layer_only_depth]

- **WHEN** a document has no document-level retained sample store but a descendant layer retains 16- or 32-bit samples
- **THEN** retained-depth is true independently of provenance; if the document records the matching high source depth and no converted source mode, a matching layer channel is authoritative at that depth; otherwise it is not

#### Scenario: Source-mode and mismatched layer stores use the fallback [dss_native_authority]

- **WHEN** a retained plane belongs to a normalized source mode, or its recorded source depth is absent or does not match, or its layer rectangle or depth does not match, or the requested channel is absent
- **THEN** the native-plane query returns no plane and consumers use their existing 8-bit path

### Requirement: Composite queries preserve storage and output distinctions

The document SHALL expose one shared predicate for a nonempty stored composite
whose dimensions match the document. Composite retention policy and RGB
four-channel shape validity SHALL remain distinct queries, preserving their
existing decisions. Merged-image output policy SHALL be independent of resident
buffer presence: computing a composite from layers SHALL NOT by itself enable or
disable writing the merged image. Shape validity SHALL NOT claim freshness.

#### Scenario: Stored-composite checks agree [dss_stored_composite]

- **WHEN** core, render, or application consumers inspect a matching nonempty composite, an absent or empty composite, or a composite whose dimensions differ from the document
- **THEN** they use the shared stored-composite predicate and respectively receive true, false, and false

#### Scenario: Shape and retention remain separate [dss_composite_roles]

- **WHEN** consumers inspect a one-plane Grayscale composite, a four-plane RGB composite, or a layered 8-bit working RGB document without retained source samples
- **THEN** the Grayscale composite can be stored without being RGB/RGBA shape-valid, the matching RGB/RGBA composite is shape-valid, and the layered working RGB document keeps its existing on-demand retention policy

#### Scenario: Merged output is independent of stored pixels [dss_merged_output]

- **WHEN** a layered document has no stored composite but merged-image output enabled, or has a maintained buffer while merged-image output is disabled
- **THEN** the application picture-save path respectively renders and writes a merged image or omits the real merged image, regardless of buffer residency. An explicit app-only write option may emit a fixture-supported placeholder or metadata; direct codec writers retain their existing default of no image-data section. Direct codec writers retain their existing composite-buffer preconditions and errors

### Requirement: State transitions preserve document data and failure atomicity

Read normalization, mode/depth conversion, profile assignment/conversion,
composite replacement/drop, geometry operations, and history restoration SHALL
use the encapsulated interface. A rejected state transition SHALL leave the
document unchanged. Successful transitions SHALL preserve the existing byte
buffers, native samples, and copy-on-write behavior except for the stores the
operation deliberately changes. Original and working profile bytes SHALL remain
independent; assigning a working profile after an incoming conversion SHALL NOT
erase the original profile record merely because both are then present.

#### Scenario: A rejected replacement leaves the document unchanged [dss_transition_atomic]

- **WHEN** an attempted replacement supplies contradictory source metadata or a composite buffer with inconsistent internal geometry
- **THEN** it is rejected before commit and the document still equals its pre-transition clone

#### Scenario: A converted document can later receive a working profile [dss_profile_transition]

- **WHEN** a document retaining an original converted-away profile is assigned another working profile
- **THEN** both profile records remain accessible, stored pixel bytes do not change, and a clone continues sharing untouched allocations

#### Scenario: History detaches and reattaches composite planes [dss_history_planes]

- **WHEN** history hollows a document and restores its tracked planes
- **THEN** composite geometry, channel metadata, output policy, and tracked-plane slot order survive, and unchanged restored planes retain their shared allocations

### Requirement: Encapsulation preserves PSD and PSB compatibility

The interface migration SHALL preserve serialized mode/depth decisions, palettes,
profile resources, retained samples, extra channels, compression selection, and
merged-image omission for existing supported documents. Existing read/write
golden and independent-oracle expectations SHALL remain unchanged; fixture or
baseline changes SHALL NOT be used to accommodate the refactor.

#### Scenario: Existing codec expectations survive the migration [dss_codec_compatibility]

- **WHEN** the existing PSD/PSB read/write, native-depth, color-mode, profile, and streamed merged-image checks run through the new state interface
- **THEN** they pass with the same expected pixels, samples, resources, output headers, and fixture bytes
