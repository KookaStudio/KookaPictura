# Document Source State Delta

## MODIFIED Requirements

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
