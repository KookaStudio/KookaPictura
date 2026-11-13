# tools/move-commit-responsiveness Specification

## Purpose
Keeps releasing a Move drag responsive on large documents: the whole-area
composite that a canvas-sized layer needs leaves the GUI thread, while the drag
preview stays presented and every reader of the document composite sees the
moved pixels.

## Requirements

### Requirement: A large layer move composites off the GUI thread

When a document keeps an authoritative composite and the document and the union
of a Move commit's dirty rectangles each reach 16 MP, the commit SHALL translate
the layer and return without compositing, and a worker SHALL composite the dirty
rectangles. The landed composite MUST equal a full composite of the moved
document. A landed whole-document region on a document over 16 MP SHALL install
level 0 and a coarse pyramid on the GUI thread and build the exact pyramid on a
worker; the canvas SHALL present the coarse level until `pyramidSettled`, and the
exact pyramid MUST equal a synchronous rebuild. A document at or under 16 MP
SHALL rebuild the pyramid synchronously. Landing or settling the commit SHALL
capture one history state labelled with the commit's label.

A layered RGB 8-bit document keeps no stored composite. When its document and
dirty union each reach 16 MP, the commit SHALL mark the dirty display tiles stale,
drop any stale stored frame, capture one history state immediately, and return
without a worker composite. The canvas SHALL rebuild the tiles it presents from
the moved layers; off-screen tiles MAY stay stale until presented. A document or
dirty union below 16 MP SHALL commit synchronously.

#### Scenario: A large commit returns before its composite [mcr_deferred]

- **WHEN** a canvas-sized layer on an authoritative document at or above 16 MP is
  committed with a dirty union at or above 16 MP
- **THEN** the commit returns with the layer moved and the composite pending, and
  when the job lands the composite equals a full composite and history gained
  one Move state

#### Scenario: The landing defers the exact pyramid [mcr_deferred_pyramid]

- **WHEN** a whole-document move composite lands on an authoritative document
  over 16 MP
- **THEN** the canvas presents a coarse level, and after `pyramidSettled` it
  presents the exact level with bytes equal to a synchronous rebuild

#### Scenario: A small commit stays synchronous [mcr_small_sync]

- **WHEN** a Move commit's document or dirty union is below 16 MP
- **THEN** the composite is current when the commit returns and nothing is
  pending

#### Scenario: A busy display stays synchronous [mcr_busy_sync]

- **WHEN** a large Move commit runs while a live session, a pending preview or a
  soloed mask owns display state
- **THEN** the commit composites synchronously

#### Scenario: A layered RGB move leaves its tiles lazy [mcr_layered_lazy]

- **WHEN** a large Move on a layered RGB 8-bit document is released without a
  test delay
- **THEN** the commit returns without a worker composite, reports
  `moveCommitted`, records one `Move Layer` state, and marks the dirty display
  tiles stale for the canvas to rebuild

### Requirement: The drag preview stays until the composite lands

A deferred Move commit SHALL leave the canvas's drag preview presented, and the
view SHALL report `moveCommitted` when the composite is current. A synchronous
commit SHALL report it before returning.

#### Scenario: The preview survives the release [mcr_preview]

- **WHEN** a large Move drag is released and the commit defers
- **THEN** the canvas still presents the drag preview, and drops it when
  `moveCommitted` arrives

### Requirement: A second Move press does not settle a pending composite

A Move press SHALL reuse a still-valid cached full-resolution preview, and SHALL
build a view preview from the layer stack while a Move composite is pending,
without settling that composite. A landing composite SHALL NOT clear a drag
preview that started after it.

#### Scenario: The second press starts without the whole composite [mcr_second_press]

- **WHEN** a large layer is moved, its composite is still pending, and the Move
  tool presses the layer again
- **THEN** the press reuses the cached preview or rebuilds the view preview
  without a whole-document composite, and the first landing does not clear the
  second drag's preview

### Requirement: A pointer readout does not settle a pending composite

The Info panel's hover readout SHALL read the maintained composite without
settling a pending Move composite, so a pointer move after a Move commit does
not block. It SHALL show the moved pixel after the next refresh once the
composite lands.

#### Scenario: Hovering after a move does not stall [mcr_hover_no_settle]

- **WHEN** the pointer moves over the canvas while a large Move composite is
  pending
- **THEN** the Info panel readout updates without a whole-document composite and
  catches up after the composite lands

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

### Requirement: A large transform commit runs as a cancellable job

A Free Transform commit SHALL resample the transform across rows in parallel,
treating pixels outside the source layer rect as transparent so an opaque edge
anti-aliases when it is rotated or scaled. The engine transform SHALL run as a
cancellable job (see `document/long-operation-progress`) when the transformed
layer rect reaches 4 MP, or when the commit gate admits a commit whose working
bytes plus spillable bytes exceed the memory governor's room, so it is expected
to pack or spill tiles. This includes a lifted selection transform and a warp.
The job's progress SHALL count the resampled output rows and SHALL keep
advancing while output tiles are written and spilled. The transform handles and
image SHALL stay presented while the job runs. On completion, the landing SHALL
install the transformed layers, record one history state under the session's
label, and leave the region composite and the exact pyramid to a worker when its
dirty union reaches 16 MP. The transform image SHALL remain presented until
`moveCommitted`. On cancel, the job SHALL land nothing, and the Free Transform
SHALL end as Esc ends it: the handles and the transform image SHALL be removed,
the layer SHALL show its original pixels, and the document and history SHALL be
unchanged. Any other transform SHALL commit synchronously, and a commit that
does not defer SHALL report `moveCommitted` before returning.

#### Scenario: A large transform release returns to Move [mcr_transform_deferred]

- **WHEN** a canvas-sized layer's Free Transform is committed on a document over 16 MP while the job is held by its test seam
- **THEN** the `jobProgress` window appears, its value rises, and on landing the history gains one `"Free Transform"` state, the transform handles are gone, and the transform image stays until `moveCommitted`

#### Scenario: Cancelling a large transform ends it [mcr_transform_cancel]

- **WHEN** a large Free Transform commit is cancelled from its progress window
- **THEN** no transform session is active, the handles and the transform image are gone, the document is pixel-identical to before the commit, and the history count is unchanged

#### Scenario: A large lifted transform runs as a job [mcr_transform_lifted_job]

- **WHEN** a Free Transform of selected pixels over 4 MP is committed and its job is cancelled, and the same session is then committed again
- **THEN** the cancelled job lands nothing and leaves the composite and selection unchanged, and the second job adds one history state with the pixels merged into the layer

#### Scenario: A small commit expected to spill runs as a job [mcr_transform_spill_job]

- **WHEN** a Free Transform of a layer under 4 MP is committed while the governor's room holds its working bytes but not its working plus spillable bytes, the scratch disks hold the rest, and the job is held by its test seam
- **THEN** the `jobProgress` window appears, and Cancel ends the transform with the document and history unchanged

#### Scenario: A small commit that fits stays synchronous [mcr_transform_fits_sync]

- **WHEN** the same commit is made with room for its working plus spillable bytes
- **THEN** no `jobProgress` window appears, and the history gains one state before the commit returns

#### Scenario: Progress advances through a spilling commit [mcr_transform_spill_progress]

- **WHEN** a 17000 × 17000 layer is scaled by 1.5 under a 2 GiB memory limit, so its output tiles spill
- **THEN** the progress window shows and its value keeps rising until the job lands

### Requirement: A large shape move defers its reach composite

When a shape layer's before-and-after reach union reaches 16 MP and the document
also reaches 16 MP, releasing a Move drag SHALL translate the shape and return
without compositing, and a worker SHALL composite the reach rectangles. The
landed composite MUST equal a full composite of the moved document, and landing
SHALL capture one history state labelled with the commit's label. A shape reach
below 16 MP, or a document below 16 MP, SHALL commit synchronously.

#### Scenario: A large shape release lands off the GUI thread [mcr_shape_deferred]

- **WHEN** a shape whose before-and-after reach union is at least 16 MP on a
  4200² document is released after a Move drag
- **THEN** the commit returns with the shape moved and the composite pending, and
  when the job lands the composite equals a full composite and history gained one
  `Move Layer` state
