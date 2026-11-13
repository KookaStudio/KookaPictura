## MODIFIED Requirements

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
