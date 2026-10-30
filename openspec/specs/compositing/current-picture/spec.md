# compositing/current-picture Specification

## Purpose
Defines how the application answers "the picture as currently shown": from the
document's maintained composite when it is fresh, and from a fresh composite of
the layer stack only when the cache cannot stand in for it.

## Requirements

### Requirement: The current picture is served from the maintained composite

The application SHALL answer a request for the picture as currently shown with
the document's maintained composite, without compositing the layer stack, when
that composite is the current picture. The maintained composite is the current
picture after every pixel-changing committed edit, every presented preview that
updates it, and every history navigation, because the edit transaction refreshes
it regionally or stores it whole for each of those. A display-only preview does
not update it (see `compositing/render-session`), so the composite cannot stand
in while one is live. An edit that changes no pixel leaves it current by
definition.

The answer SHALL borrow the document's composite rather than copy it, so serving
the current picture costs no document-sized allocation.

#### Scenario: A committed edit leaves the composite current [cp_after_edit]

- **WHEN** a pixel-changing edit is committed through the edit transaction and the current picture is requested
- **THEN** the answer is the document's composite, not a newly composited stack

#### Scenario: A presented preview leaves the composite current [cp_after_preview]

- **WHEN** a live preview that updates the document composite (filter, a bounded or masked adjustment, opacity, fill, or mask refinement) has been presented and the current picture is requested
- **THEN** the answer is the document's composite, which already includes the preview

#### Scenario: History navigation leaves the composite current [cp_after_history]

- **WHEN** undo, redo, or a history jump has restored a document state and the current picture is requested
- **THEN** the answer is the restored document's composite, not a newly composited stack

### Requirement: The current picture is composited when the composite is not current

The application SHALL composite the layer stack for the current picture when the
maintained composite cannot stand in for it, and SHALL answer with the document's
embedded composite when the layer stack is empty. The composite cannot stand in
when:

- a live stroke is writing the layer pixels or the mask coverage in place (a
  layer stroke's composite lags its unpresented regions, and a mask stroke
  deliberately leaves the composite at its pre-stroke value);
- a display-only preview is live (it presents an approximate frame and the
  maintained composite predates it);
- a lifted transform is live (lifting moves the selection's pixels into a
  floating layer and clears the source, so the composite predates the lift);
- the document's working mode is not RGB (its composite keeps the document's own
  colour planes, not RGBA);
- the document retains a native-depth composite, which a save may rewrite to
  the native narrowing rather than the 8-bit composite these consumers read;
- the composite does not have four channels; or
- the composite's dimensions differ from the document's.

#### Scenario: A live layer stroke is not served from the composite [cp_layer_stroke]

- **WHEN** a paint stroke is live and the current picture is requested
- **THEN** the answer is a fresh composite of the layer stack, not the maintained composite that lags the stroke

#### Scenario: A live mask stroke is not served from the composite [cp_mask_stroke]

- **WHEN** a mask stroke is live and the current picture is requested
- **THEN** the answer is a fresh composite of the layer stack, not the maintained pre-stroke composite

#### Scenario: A display-only preview is not served from the composite [cp_display_preview]

- **WHEN** a display-only preview is live and the current picture is requested
- **THEN** the answer is a fresh composite of the layer stack, not the maintained composite that predates the preview

#### Scenario: A lifted transform is not served from the composite [cp_lifted_transform]

- **WHEN** a lifted transform is live and the current picture is requested
- **THEN** the answer is a fresh composite of the lifted layer stack, not the composite from before the lift

#### Scenario: A non-RGB document is composited [cp_non_rgb]

- **WHEN** the document's working mode is not RGB and the current picture is requested
- **THEN** the answer is a four-channel composite of the layer stack, not the document's colour-plane composite

#### Scenario: A mismatched composite is not served [cp_size_mismatch]

- **WHEN** a document with layers has a composite whose dimensions differ from the document's
- **THEN** the answer is a fresh composite at the document's dimensions

#### Scenario: A layerless document keeps its embedded composite [cp_layerless]

- **WHEN** the document has no layers and the current picture is requested
- **THEN** the answer is the document's embedded composite, not an empty layer-stack composite

### Requirement: The served picture is byte-identical to a full composite

Every answer to a request for the current picture SHALL equal, sample for
sample, the buffer the existing full-composite path produces for the same state:
a composite of the layer stack, or the embedded composite when the layer stack is
empty.

#### Scenario: The served frame matches a full composite [cp_identical]

- **WHEN** the current picture is requested for a fresh, a previewed, a live-stroke, a lifted-transform, a non-RGB, a layerless, and a mismatched-composite document
- **THEN** every answer equals the full-composite path's buffer for that state sample for sample

### Requirement: Read-only consumers read the current picture

A consumer that only reads the picture as currently shown — selection sampling,
paint-tool sampling, clipboard copy-merged, image adjustments, and raster export
or save — SHALL read it through the current-picture accessor and MUST NOT run the
compositor while the composite is current.

#### Scenario: A read-only consumer composites nothing on a fresh document [cp_consumer]

- **WHEN** a read-only consumer samples a fresh document whose composite is current
- **THEN** it reads the document's composite and no compositor runs
