# tools/move-commit-responsiveness Specification

## Purpose
Keeps releasing a Move drag responsive on large documents: the whole-area
composite that a canvas-sized layer needs leaves the GUI thread, while the drag
preview stays presented and every reader of the document composite sees the
moved pixels.

## Requirements

### Requirement: A large layer move composites off the GUI thread

When the document and the union of a Move commit's dirty rectangles each reach
16 MP, the commit SHALL translate the layer and return without compositing, and
a worker SHALL composite the dirty rectangles. The landed composite MUST equal a
full composite of the moved document. Landing or settling the commit SHALL
capture one history state labelled with the commit's label. A document or dirty
union below 16 MP SHALL commit synchronously.

#### Scenario: A large commit returns before its composite [mcr_deferred]

- **WHEN** a canvas-sized layer on a document at or above 16 MP is committed
  with a dirty union at or above 16 MP
- **THEN** the commit returns with the layer moved and the composite pending, and
  when the job lands the composite equals a full composite and history gained
  one Move state

#### Scenario: A small commit stays synchronous [mcr_small_sync]

- **WHEN** a Move commit's document or dirty union is below 16 MP
- **THEN** the composite is current when the commit returns and nothing is
  pending

#### Scenario: A busy display stays synchronous [mcr_busy_sync]

- **WHEN** a large Move commit runs while a live session, a pending preview or a
  soloed mask owns display state
- **THEN** the commit composites synchronously

### Requirement: The drag preview stays until the composite lands

A deferred Move commit SHALL leave the canvas's drag preview presented, and the
view SHALL report `moveCommitted` when the composite is current. A synchronous
commit SHALL report it before returning.

#### Scenario: The preview survives the release [mcr_preview]

- **WHEN** a large Move drag is released and the commit defers
- **THEN** the canvas still presents the drag preview, and drops it when
  `moveCommitted` arrives

### Requirement: Every reader settles a pending composite

Before an edit, a preview, a Move press, undo, redo, a history jump, a save, an
eyedropper sample or a filter proxy reads or writes `doc.composite`, a pending
Move composite SHALL be finished synchronously. The picture accessor SHALL treat
a pending composite as not current. A settle SHALL capture the pending move's
history state exactly once, before the reader's own work.

#### Scenario: An edit settles before its own capture [mcr_settle_edit]

- **WHEN** an edit or a Move press begins while a Move composite is pending
- **THEN** the pending composite and its history state land first, and the new
  edit starts from the moved document

#### Scenario: Undo sees the pending move [mcr_settle_undo]

- **WHEN** undo, redo or a history jump runs while a Move composite is pending
- **THEN** the pending move is settled first, undo is offered while it is
  pending, and redo is not

#### Scenario: Save and sampling read the moved pixels [mcr_settle_read]

- **WHEN** a save, a plain eyedropper sample or the picture accessor runs while a
  Move composite is pending
- **THEN** the composite is current, equal to a full composite of the moved
  document

### Requirement: A superseded or abandoned job is dropped

A worker result SHALL land only while its generation is pending. A settle, a
later commit, or a document replacement SHALL drop an older result, and a
replacement MUST NOT be patched by a job from the outgoing document.

#### Scenario: A stale result is dropped [mcr_stale]

- **WHEN** a Move composite job lands after a settle or a later commit
- **THEN** its result is dropped and the current document is unchanged by it

#### Scenario: A replacement drops the job [mcr_replace]

- **WHEN** the document is replaced while a Move composite is pending
- **THEN** nothing is pending afterwards and the late result cannot patch the new
  document

### Requirement: The large-document release stays responsive

On a real display, releasing a Move drag of a canvas-sized layer on a
16507×16196 document SHALL return to the event loop without a visible stall, and
the layer SHALL show its moved pixels once the composite lands.

#### Scenario: The pointer stays responsive after release [mcr_large_manual]

- **WHEN** on a real display, a canvas-sized layer on a 16507×16196 document is
  dragged and released at fit zoom and at 100 %
- **THEN** the release does not block the GUI thread, the drag preview remains
  until the composite lands, and the layer then shows its moved pixels
