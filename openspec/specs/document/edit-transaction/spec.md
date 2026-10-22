# document/edit-transaction Specification

## Purpose
One owner for committing a document edit: the mutation declares what changed, and the transaction performs compositing, linkage invalidation, history capture, revision, and dirty bookkeeping in one fixed order.

## Requirements

### Requirement: One transaction commits every document edit

The app SHALL commit every document edit through a single edit-transaction interface that takes a history label and a mutation. The mutation SHALL run inside the transaction, so that one transaction owns the document change and its history state: a mutation that reports unchanged SHALL commit no history state. A mutation that refuses MUST detect the refusal before it writes, so the refused edit leaves the document unchanged and the transaction captures no history state for it. The mutation SHALL declare one outcome:

- unchanged;
- metadata-only or otherwise non-compositing (selection, name, lock, colour label, channel list, guides, annotations, slices, file metadata, or resolution);
- already-presented (a preview already presented the change);
- one or more bounded document rectangles;
- the whole document;
- a link-invalidating change.

The mutation SHALL report the outcome of the change it made. The transaction SHALL derive the compositing, linkage, history, revision, and dirty work from that outcome and SHALL return the outcome to the caller. Command code MUST NOT hand-write the refresh-then-capture sequence.

#### Scenario: A committed edit is refreshed and captured once [et_one_owner]

- **WHEN** a command commits a document change through the transaction
- **THEN** the declared compositing runs, exactly one history state is captured, and the document is left dirty with its content revision advanced

#### Scenario: The outcome is reported to the caller [et_reports_outcome]

- **WHEN** the mutation returns an outcome
- **THEN** the transaction returns the same outcome so the command can report whether and how the document changed

#### Scenario: A failed mutation leaves no change and no history [et_mutation_inside]

- **WHEN** a mutation runs inside the transaction and refuses before writing, or changes nothing
- **THEN** the document is unchanged, no history state is captured, and no change signal is emitted

### Requirement: An unchanged mutation does no work

When the mutation reports that nothing changed, the transaction SHALL capture no history state, run no compositor, mark no damage, leave the content revision and dirty flag unchanged, and emit no signal.

#### Scenario: A refused edit is a no-op [et_noop]

- **WHEN** a mutation reports unchanged (a refused command or an out-of-range target)
- **THEN** the history depth, content revision, composite, damage, and dirty flag are unchanged and no compositing or change signal is emitted

### Requirement: The declared outcome selects the refresh path

A metadata-only outcome SHALL capture one history state and notify without running the compositor or marking display damage. An already-presented outcome SHALL capture one history state without a new notification or compositor run. A bounded outcome SHALL refresh only the declared rectangles through the regional path and emit one region blit, leave every pixel outside them unchanged, and equal a full recomposite. It MAY also request the command's existing change notification, which the transaction emits once after capture; visibility toggles, paint commits, and previews do not request it, while bounded property commits do; a bounded outcome with several disjoint rectangles SHALL composite them in one pass and emit one blit over their bounding box. An unbounded outcome SHALL run one full recomposite and notify, and SHALL NOT run a region blit. A link-invalidating outcome SHALL clear the positional link sets, run one full recomposite, and notify; it includes layer-tree edits and the non-tree commands that currently drop links (Pictura Raw, type conversion and type styles, rasterizing a shape, and free transform).

#### Scenario: A bounded edit refreshes only its rectangles [et_bounded]

- **WHEN** an edit declares one or more bounded rectangles
- **THEN** only those rectangles are composited, pixels outside them keep their previous value, and the result equals a full recomposite

#### Scenario: A multi-rectangle paint commit emits one blit [et_paint_regions]

- **WHEN** a paint or mask commit declares several disjoint dirty rectangles
- **THEN** the regions are composited in one pass and exactly one region blit is emitted over their bounding box

#### Scenario: A metadata edit records without compositing [et_meta]

- **WHEN** an edit changes only metadata or the selection
- **THEN** one history state is captured, no compositor runs, no display damage is marked, and one change notification is emitted

#### Scenario: An already-presented edit records without notifying [et_record]

- **WHEN** a preview already presented the change and the commit declares already-presented
- **THEN** one history state is captured, no compositor runs, and no additional change notification is emitted

#### Scenario: A link-invalidating edit rebuilds and drops links [et_structural]

- **WHEN** an edit invalidates positional layer links
- **THEN** the link sets are cleared, the whole document is recomposited, and the whole canvas is rebuilt

#### Scenario: An unbounded pixel edit rebuilds fully [et_unbounded]

- **WHEN** an edit's pixel effect cannot be bounded to a rectangle
- **THEN** the whole document is recomposited once and the result equals the refreshed display

### Requirement: History is captured after the composite is current

The transaction SHALL composite before it captures the history state, so the captured document carries the current composite. Exactly one state SHALL be captured per committed edit. A preview edit SHALL capture nothing; the commit that finishes the preview SHALL capture exactly one state. A commit that only moves the topmost pixel layer SHALL NOT advance the content revision, so a cached move preview stays valid.

#### Scenario: A commit captures the refreshed state [et_capture_after_refresh]

- **WHEN** a committed edit is undone
- **THEN** the restored document and display are byte-identical to the pre-edit state and the undo path runs no full composite

#### Scenario: A preview records nothing until its commit [et_preview]

- **WHEN** a preview edit runs repeatedly and is then committed
- **THEN** the previews add no history states and the commit adds exactly one

#### Scenario: A move commit keeps the content revision [et_move_revision]

- **WHEN** the topmost pixel layer is moved and committed
- **THEN** one history state is captured and the content revision is unchanged

### Requirement: Engine document operations do not composite

Geometry document operations — resize, canvas resize, crop, delete-cropped-pixels, orientation, layer translation, arbitrary rotation, and perspective crop — SHALL leave the document's composite untouched. The app SHALL perform the single recomposite through the transaction. `translate_layer_active` SHALL remain the CPU/GPU composite oracle. Colour-mode, bitmap, indexed, and HDR conversions SHALL continue to write the composite as part of the conversion; an 8/16-bit depth change SHALL preserve the existing display composite.

#### Scenario: A geometry operation leaves the composite for the app [et_engine_no_composite]

- **WHEN** a resize, canvas-size change, crop, orientation, translation, arbitrary rotation, or perspective crop returns
- **THEN** the engine has not run the compositor and the app's single refresh produces the displayed result

#### Scenario: A mode conversion still owns its composite [et_mode_composite]

- **WHEN** the document's colour mode is converted, or an HDR or bitmap/indexed conversion runs
- **THEN** the conversion writes the composite in the converted representation, and the transaction treats the edit as a full refresh

#### Scenario: A bit-depth change keeps the display composite [et_depth_composite]

- **WHEN** the document is converted between 8 and 16 bits per channel
- **THEN** the existing 8-bit display composite is preserved and the transaction does not expect the conversion to rebuild it

### Requirement: A committed edit presents the change once

A committed edit whose outcome notifies SHALL present the change to the shell exactly once. An outcome that does not notify — a bounded region without notification, or an already-presented commit — SHALL present nothing from the transaction. A command handler MUST NOT present a document edit again when every successful outcome its command can commit notifies. A command handler whose command can commit a successful outcome that does not notify SHALL keep its own presentation. A command that changes only metadata or only the selection SHALL present its change, so the history state and the command table reflect it without the caller asking for a refresh.

#### Scenario: A committed edit notifies once [et_notify_once]

- **WHEN** a command commits a document edit whose outcome notifies
- **THEN** the shell is notified exactly once, and no second notification follows it for that edit

#### Scenario: A handler presents nothing after a presented edit [et_no_double_present]

- **WHEN** a menu command commits a document edit that the transaction has already presented
- **THEN** the command handler presents nothing further, and the window is refreshed once for that edit

#### Scenario: A metadata edit presents its change [et_meta_presented]

- **WHEN** a command commits a metadata-only edit, such as saving a selection into a channel
- **THEN** the shell is notified, so the history state and the command table reflect the change without the caller asking for a refresh
