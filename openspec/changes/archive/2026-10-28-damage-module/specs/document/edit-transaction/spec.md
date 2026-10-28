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

### Requirement: The declared outcome selects the refresh path

A metadata-only outcome SHALL capture one history state and notify without running the compositor or marking display damage. An already-presented outcome SHALL capture one history state without a new notification or compositor run. A bounded outcome SHALL refresh only the declared rectangles through the regional path and emit one region blit, leave every pixel outside them unchanged, and equal a full recomposite. It MAY also request the command's existing change notification, which the transaction emits once after capture; visibility toggles, paint commits, and previews do not request it, while bounded property commits do. A bounded outcome that changes layer-panel or command-registry state (batch property/visibility/clip/mask/style edits, mask-refine commits, and smart-filter toggles) SHALL request the notification. A bounded outcome with several disjoint rectangles SHALL composite them in one pass and emit one blit over their bounding box. An empty rectangle list declares no bounded rectangle and SHALL use the full path.

An unbounded outcome SHALL run one full recomposite and notify, and SHALL NOT run a region blit. A link-invalidating outcome SHALL clear the positional link sets and notify. When it carries a bounded damage it SHALL refresh only those rectangles through the regional path and emit one region blit; otherwise it SHALL run one full recomposite. It SHALL keep the full path, regardless of a declared damage, when the damage area is at least three quarters of the canvas, a live gesture or pending preview is active, or a mask is soloed. It includes layer-tree edits and the non-tree commands that currently drop links (Pictura Raw, type conversion and type styles, rasterizing a shape, and free transform). A content-neutral insertion outcome, declared only for a layer or group insertion that `compositing/described-damage` classifies as content-neutral, SHALL clear the positional link sets, capture one history state, and notify. It SHALL NOT run the compositor, rebuild the display pyramid, or mark display damage. When a mask is soloed, it SHALL also re-resolve the solo source.

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
- **THEN** the transaction emits exactly one change notification and the panel and command registry refresh

#### Scenario: A link-invalidating edit rebuilds and drops links [et_structural]

- **WHEN** an edit invalidates positional layer links and declares a bounded damage
- **THEN** the link sets are cleared, only the declared rectangles are composited, one region blit is emitted, and one change notification is emitted

#### Scenario: A link-invalidating edit without a bound rebuilds fully [et_structural_full]

- **WHEN** an edit invalidates positional layer links and declares no bounded damage
- **THEN** the link sets are cleared, one full recomposite runs, and the whole canvas is rebuilt

#### Scenario: An unbounded pixel edit rebuilds fully [et_unbounded]

- **WHEN** an edit's pixel effect cannot be bounded to a rectangle
- **THEN** the whole document is recomposited once and the result equals the refreshed display

#### Scenario: A content-neutral insertion drops links without compositing [et_inserted]

- **WHEN** a content-neutral layer insertion commits
- **THEN** the link sets are cleared, one history state is captured, one change notification is emitted, and the composite, the display pyramid, and the frame revision are unchanged

## ADDED Requirements

### Requirement: History capture receives the edit's changed rectangles

The transaction SHALL pass the edit's changed document rectangles to the history capture whenever the edit declares them, including a bounded region edit, a move commit, and a link-invalidating edit with bounded damage. A paint commit SHALL pass every rectangle the stroke changed, including regions it presented before the commit, not only the region still pending. An edit that declares no bounded rectangle or an empty rectangle list SHALL pass no hint, so history compares every tracked plane in full. A structural edit that falls back to the full path SHALL pass no hint. A bounded edit that commits while an uncommitted opacity, fill, or mask preview is pending SHALL pass no hint. A mask-refine commit that finishes an earlier preview SHALL pass no hint unless it includes the union of the accumulated preview damage.

#### Scenario: A bounded edit hints its rectangles [et_history_hint]

- **WHEN** a bounded edit commits
- **THEN** the history capture receives the same rectangles the edit refreshed

#### Scenario: A move commit hints its rectangles [et_history_hint_move]

- **WHEN** a bounded move commit records with the move's source/destination union
- **THEN** the history capture receives that union even though the commit does not advance the content revision

#### Scenario: An empty rectangle list is no hint [et_history_hint_empty]

- **WHEN** an edit declares an empty rectangle list
- **THEN** the history capture receives no hint and compares every tracked plane in full

#### Scenario: A paint commit hints the whole stroke [et_history_hint_stroke]

- **WHEN** a paint stroke presented some or all of its regions before the commit and commits the rest
- **THEN** the history capture receives the whole stroke's changed rectangles, not only the pending region
