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

A layered RGB 8-bit document keeps no stored composite. When its document
reaches 16 MP and the display refresh leaves any presented tile of the dirty
union stale (a dirty union over the 4 MP repair budget, or tiles never built),
the commit SHALL drop any stale stored frame, capture one history state
immediately, keep the drag preview presented, request the presented display
tiles, and return without a worker composite, whatever the size of the dirty
union. The view SHALL report `moveCommitted` when those presented tiles are
current. The canvas SHALL show the moved layers through the preview until then;
off-screen tiles MAY stay stale until presented. A Move committed while an
earlier lazy Move still waits SHALL inherit that wait without reporting the
earlier Move, so the canvas keeps the new drag's preview. A lazy Move with a
view preview SHALL paint that preview's result into the presented level's
stale tiles as their stand-in, leaving them stale for the rebuild. A lazy
transform commit, or a lazy Move without a view preview, SHALL paint a reduced
composite of its changed area instead when a reduced copy draws the document
faithfully; a transform commit that did so SHALL report `moveCommitted` at once. A commit whose
presented tiles stay current, and a document below 16 MP, SHALL commit
synchronously.

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

- **WHEN** a Move commit's document is below 16 MP, or its dirty union is below
  16 MP and leaves every presented tile current
- **THEN** the composite is current when the commit returns and nothing is
  pending

#### Scenario: A busy display stays synchronous [mcr_busy_sync]

- **WHEN** a large Move commit runs while a live session, a pending preview or a
  soloed mask owns display state
- **THEN** the commit composites synchronously

#### Scenario: A layered RGB move leaves its tiles lazy [mcr_layered_lazy]

- **WHEN** a large Move on a layered RGB 8-bit document is released without a
  test delay
- **THEN** the commit returns without a worker composite, keeps the preview
  presented, requests the presented tiles, records one `Move Layer` state, and
  reports `moveCommitted` only when those tiles are current

#### Scenario: The moved layer shows before its tiles land [mcr_layered_preview]

- **WHEN** the same Move is released and the canvas is inspected before the
  presented tiles finish rebuilding
- **THEN** the moved layer is at its new position, its old position shows the
  backdrop, and the preview drops when `moveCommitted` arrives

#### Scenario: A second move inherits the pending wait [mcr_lazy_chain]

- **WHEN** a second Move on a large layered RGB 8-bit document is released
  before the first Move's presented tiles land
- **THEN** no `moveCommitted` is reported for the first Move, the canvas keeps
  the second drag's preview, the stale presented tiles already show the first
  Move's result so no older position shows around it, and one `moveCommitted`
  arrives when the presented tiles are current

#### Scenario: A layered transform shows its result at once [mcr_transform_lazy]

- **WHEN** a Free Transform is committed on a large layered RGB 8-bit document
  whose presented tiles it leaves stale, and a reduced copy draws it faithfully
- **THEN** the stale presented tiles show a reduced composite of the result,
  `moveCommitted` is reported at once so the canvas drops the untransformed
  preview, and the tiles still rebuild exactly

#### Scenario: A mid-sized layered move waits for its tiles [mcr_layered_mid]

- **WHEN** a Move on a layered RGB 8-bit document of at least 16 MP is released
  with a dirty union between the 4 MP repair budget and 16 MP
- **THEN** the commit keeps the preview presented and reports `moveCommitted`
  only when the presented tiles are current, so the old position never shows
