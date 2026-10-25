# Spec Delta

## MODIFIED Requirements

### Requirement: The declared outcome selects the refresh path

A metadata-only outcome SHALL capture one history state and notify without running the compositor or marking display damage. An already-presented outcome SHALL capture one history state without a new notification or compositor run. A bounded outcome SHALL refresh only the declared rectangles through the regional path and emit one region blit, leave every pixel outside them unchanged, and equal a full recomposite. It MAY also request the command's existing change notification, which the transaction emits once after capture; visibility toggles, paint commits, and previews do not request it, while bounded property commits do; a bounded outcome with several disjoint rectangles SHALL composite them in one pass and emit one blit over their bounding box. An unbounded outcome SHALL run one full recomposite and notify, and SHALL NOT run a region blit. A link-invalidating outcome SHALL clear the positional link sets, run one full recomposite, and notify; it includes layer-tree edits and the non-tree commands that currently drop links (Pictura Raw, type conversion and type styles, rasterizing a shape, and free transform). A content-neutral insertion outcome, declared only for a layer or group insertion that `compositing/described-damage` classifies as content-neutral, SHALL clear the positional link sets, capture one history state, and notify. It SHALL NOT run the compositor, rebuild the display pyramid, or mark display damage. When a mask is soloed, it SHALL also re-resolve the solo source.

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

#### Scenario: A content-neutral insertion drops links without compositing [et_inserted]

- **WHEN** a content-neutral layer insertion commits
- **THEN** the link sets are cleared, one history state is captured, one change notification is emitted, and the composite, the display pyramid, and the frame revision are unchanged
