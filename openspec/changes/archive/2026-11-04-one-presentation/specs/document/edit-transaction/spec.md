# Spec Delta

## MODIFIED Requirements

### Requirement: One transaction commits every document edit

The app SHALL commit every document edit through a single edit-transaction interface that takes a history label and a mutation. The mutation SHALL run inside the transaction, so that one transaction owns the document change and its history state: a mutation that reports unchanged SHALL commit no history state. A mutation that refuses MUST detect the refusal before it writes, so the refused edit leaves the document unchanged and the transaction captures no history state for it. The mutation SHALL declare one outcome:

- unchanged;
- metadata-only or otherwise non-compositing (selection, name, lock, colour label, channel list, guides, annotations, slices, file metadata, or resolution);
- already-presented (a preview already presented the change);
- one or more bounded document rectangles;
- the whole document;
- a link-invalidating change, optionally with a bounded damage.

The mutation SHALL report the outcome of the change it made. The transaction SHALL derive the compositing, linkage, and history work from that outcome, SHALL return the normalized outcome to the caller, and SHALL present the change through the one presenter, which owns the dirty and content-revision writes. Command code MUST NOT hand-write the refresh-then-capture sequence.

#### Scenario: A committed edit is refreshed and captured once [et_one_owner]

- **WHEN** a command commits a document change through the transaction
- **THEN** the declared compositing runs, exactly one history state is captured, and the document is left dirty with its content revision advanced

#### Scenario: The outcome is reported to the caller [et_reports_outcome]

- **WHEN** the mutation returns an outcome
- **THEN** the transaction returns the same normalized outcome so the command can report whether and how the document changed

#### Scenario: A failed mutation leaves no change and no history [et_mutation_inside]

- **WHEN** a mutation runs inside the transaction and refuses before writing, or changes nothing
- **THEN** the document is unchanged, no history state is captured, and no change signal is emitted

### Requirement: The declared outcome selects the refresh path

A metadata-only outcome SHALL capture one history state and notify without running the compositor or marking display damage. An already-presented outcome SHALL capture one history state without a new notification or compositor run. A bounded outcome SHALL refresh only the declared rectangles through the regional path, report them, leave every pixel outside them unchanged, and equal a full recomposite; the presenter SHALL emit one region blit over their bounding box. It MAY also request the command's existing change notification, which the presenter emits once after capture; visibility toggles, paint commits, and previews do not request it, while bounded property commits do. A bounded outcome that changes layer-panel or command-registry state (batch property/visibility/clip/mask/style edits, mask-refine commits, and smart-filter toggles) SHALL request the notification. A bounded outcome with several disjoint rectangles SHALL composite them in one pass and report one region over their bounding box. An empty rectangle list declares no bounded rectangle and SHALL use the full path.

An unbounded outcome SHALL compute one full document composite, leave it exact, and request a rebuild; the presenter SHALL rebuild the display once from that composite and notify, and SHALL NOT run a region blit. A link-invalidating outcome SHALL clear the positional link sets and notify. When it carries a bounded damage it SHALL refresh only those rectangles through the regional path and report one region; otherwise it SHALL compute one full document composite and request a rebuild. It SHALL keep the full path, regardless of a declared damage, when the damage area is at least three quarters of the canvas, a live gesture or pending preview is active, or a mask is soloed. It includes layer-tree edits and the non-tree commands that currently drop links (Pictura Raw, type conversion and type styles, rasterizing a shape, and free transform). A content-neutral insertion outcome, declared only for a layer or group insertion that `compositing/described-damage` classifies as content-neutral, SHALL clear the positional link sets, capture one history state, and notify. It SHALL NOT run the compositor, rebuild the display pyramid, or mark display damage. When a mask is soloed, it SHALL also re-resolve the solo source.

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

#### Scenario: A panel-mutating bounded edit notifies [et_bounded_notify]

- **WHEN** a bounded batch property, mask-refine commit, or smart-filter toggle changes layer-panel or command-registry state
- **THEN** the presenter emits exactly one change notification and the panel and command registry refresh

#### Scenario: A link-invalidating edit rebuilds and drops links [et_structural]

- **WHEN** an edit invalidates positional layer links and declares a bounded damage
- **THEN** the link sets are cleared, only the declared rectangles are composited, one region blit is emitted, and one change notification is emitted

#### Scenario: A link-invalidating edit without a bound rebuilds fully [et_structural_full]

- **WHEN** an edit invalidates positional layer links and declares no bounded damage
- **THEN** the link sets are cleared, one full document composite is computed, and the presenter rebuilds the whole display once

#### Scenario: An unbounded pixel edit rebuilds fully [et_unbounded]

- **WHEN** an edit's pixel effect cannot be bounded to a rectangle
- **THEN** the whole document is composited once and the presenter rebuilds the display so the result equals the refreshed display

#### Scenario: A content-neutral insertion drops links without compositing [et_inserted]

- **WHEN** a content-neutral layer insertion commits
- **THEN** the link sets are cleared, one history state is captured, one change notification is emitted, and the composite, the display pyramid, and the frame revision are unchanged

### Requirement: History is captured after the composite is current

The transaction SHALL composite before it captures the history state, so the captured document carries the current composite. Exactly one state SHALL be captured per committed edit. A preview edit SHALL capture nothing; the commit that finishes the preview SHALL capture exactly one state. A commit that only moves the topmost pixel layer SHALL NOT advance the content revision, so a cached move preview stays valid. The presenter SHALL apply the reported content-revision flag after capture; history capture MUST NOT write it.

#### Scenario: A commit captures the refreshed state [et_capture_after_refresh]

- **WHEN** a committed edit is undone
- **THEN** the restored document and display are byte-identical to the pre-edit state and the undo path runs no full composite

#### Scenario: A preview records nothing until its commit [et_preview]

- **WHEN** a preview edit runs repeatedly and is then committed
- **THEN** the previews add no history states and the commit adds exactly one

#### Scenario: A move commit keeps the content revision [et_move_revision]

- **WHEN** the topmost pixel layer is moved and committed
- **THEN** one history state is captured and the content revision is unchanged

### Requirement: A committed edit presents the change once

The presenter SHALL present a committed edit whose outcome notifies to the shell exactly once. An outcome that does not notify — a bounded region without notification, or an already-presented commit — SHALL present nothing from the transaction. A command handler MUST NOT present a document edit again when every successful outcome its command can commit notifies. A command handler whose command can commit a successful outcome that does not notify SHALL keep its own presentation. A command that changes only metadata or only the selection SHALL present its change, so the history state and the command table reflect it without the caller asking for a refresh.

#### Scenario: A committed edit notifies once [et_notify_once]

- **WHEN** a command commits a document edit whose outcome notifies
- **THEN** the shell is notified exactly once, and no second notification follows it for that edit

#### Scenario: A handler presents nothing after a presented edit [et_no_double_present]

- **WHEN** a menu command commits a document edit that the transaction has already presented
- **THEN** the command handler presents nothing further, and the window is refreshed once for that edit

#### Scenario: A metadata edit presents its change [et_meta_presented]

- **WHEN** a command commits a metadata-only edit, such as saving a selection into a channel
- **THEN** the shell is notified, so the history state and the command table reflect the change without the caller asking for a refresh
