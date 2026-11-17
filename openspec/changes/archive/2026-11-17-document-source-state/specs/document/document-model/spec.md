# Spec Delta

## MODIFIED Requirements

### Requirement: Document and layer clones share their storage

Cloning a `Document` or `Layer` SHALL be a refcount bump for every bulk store it
owns: pixel planes (including those held by the encapsulated composite value),
retained native sample stores, smart-object payloads and the raw linked-section
bytes (`Document.layer_section_extra`), preserved byte blocks (including original
and working ICC bytes held by the encapsulated source value,
`Document.color_mode_data`, `Document.image_resources`,
`Document.global_layer_mask`, `Layer.blending_ranges`, `SmartFilter.options`, and
the `TypeTool` descriptors), and the layer tree. A clone MUST NOT copy those
buffers. A write SHALL fork only the store it touches; untouched stores SHALL
remain shared. Layer-tree structural edits SHALL fork the tree once, leaving the
source tree's nodes unchanged. Read access to source and composite buffers SHALL
borrow their stores without allocating replacement byte buffers.

#### Scenario: Cloning a document shares native samples and payload

- **WHEN** a document carrying a 16-bit sample store and an embedded smart-object payload is cloned
- **THEN** the clone shares both allocations and equals the original field for field

#### Scenario: A smart-object payload and its raw linked section share

- **WHEN** a document with an embedded smart object is cloned
- **THEN** the clone shares `SmartObject.payload` and `Document.layer_section_extra` with the original, and both remain byte-identical

#### Scenario: A layer clone shares its byte blocks and layer tree

- **WHEN** a layer with `extra_blocks`, `raw_channels`, `blending_ranges`, a mask, adjustment data, smart-filter options, type descriptors, and child layers is cloned
- **THEN** the clone shares those block and child-layer allocations, and a later write to either side does not change the other

#### Scenario: Preserved document byte blocks share on clone

- **WHEN** a document with original ICC bytes, working ICC bytes, `color_mode_data`, `image_resources`, `global_layer_mask`, and `layer_section_extra` is cloned
- **THEN** borrowed access through each clone exposes the same respective allocations, and the bytes are unchanged

#### Scenario: A structural edit forks the tree

- **WHEN** a layer is added, removed, or reordered in a cloned document
- **THEN** the edit forks the layer tree once and the other document's tree is unchanged

#### Scenario: A written store forks alone

- **WHEN** a shared sample, payload, raw linked-section, or byte-block store is written
- **THEN** only that store is copied; every other shared store keeps its allocation
