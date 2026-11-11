## MODIFIED Requirements

### Requirement: The current picture is served from the maintained composite

The application SHALL answer a request for a region of the picture as currently
shown from the maintained composite when that composite is the current picture,
without compositing the layer stack again for any cached tile. For a layered RGB
document the maintained composite is the tile pyramid cache: its tiles are
repaired by every pixel-changing committed edit, every presented preview that
updates the picture, and every history navigation through their damage, and a
missing tile is composited for its region only. For a document whose composite is
authoritative (no layers, a non-RGB mode, or a retained native-depth merged
composite) the maintained composite is the document's stored composite. A
display-only preview does not update it (see `compositing/render-session`), so it
cannot stand in while one is live. An edit that changes no pixel leaves it current
by definition.

Serving a region SHALL cost an allocation of that region's size, not a
document-sized allocation.

#### Scenario: A committed edit leaves the composite current [cp_after_edit]

- **WHEN** a pixel-changing edit is committed through the edit transaction and a region of the current picture is requested
- **THEN** the answer is served from the maintained composite, not a newly composited stack

#### Scenario: A presented preview leaves the composite current [cp_after_preview]

- **WHEN** a live preview that updates the picture (filter, a bounded or masked adjustment, opacity, fill, or mask refinement) has been presented and a region of the current picture is requested
- **THEN** the answer is served from the maintained composite, which already includes the preview

#### Scenario: History navigation leaves the composite current [cp_after_history]

- **WHEN** undo, redo, or a history jump has restored a document state and a region of the current picture is requested
- **THEN** the answer is served from the maintained composite of the restored state, not a newly composited whole stack

### Requirement: Read-only consumers read the current picture

A consumer that only reads the picture as currently shown — selection sampling,
paint-tool sampling, clipboard copy-merged, image adjustments, and raster export
or save — SHALL read it through the current-picture accessor, as regions or
streamed bands where the consumer can work by region, and MUST NOT composite a
cached region again while the maintained composite is current.

#### Scenario: A read-only consumer composites nothing on a fresh document [cp_consumer]

- **WHEN** a read-only consumer samples a region of a fresh document whose maintained composite is current and cached
- **THEN** it reads the cached composite and no compositor runs
