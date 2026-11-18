

## REMOVED Requirements

### Requirement: An unavailable preview falls back without lying

**Reason**: Stored-image availability is not the availability of a logical tile level.

**Migration**: Use An unavailable tile-level preview falls back without lying; existing scenario tags remain traceable in the replacement.

## ADDED Requirements

### Requirement: An unavailable tile-level preview falls back without lying

When a preview cannot be served — an unavailable requested level, no presented
level, an unsummarisable layer prefix, a soloed mask, a non-RGB or native-depth
document, a live stroke, a reduced-level request for content that cannot be reduced (a live
type layer, a proxy-less smart object, or a vector fill), or a backdrop plus scratch that would
exceed the session's memory budget — the application SHALL use the existing exact
path. An empty visible rectangle SHALL present nothing and mark no damage. It
MUST NOT present an approximate frame as the exact composite. The absence of
a whole stored level SHALL NOT itself make a supported tile-pyramid level
unavailable. A supported level-0 preview SHALL remain available even when the
document has no reduced level, and its frame SHALL be exact.

#### Scenario: An unavailable requested level falls back [rs_fallback_no_level]

- **WHEN** the canvas requests a level unavailable in the document's pyramid
- **THEN** the exact path runs and the presented frame equals the full composite

#### Scenario: A level-0-only document previews exactly [rs_level0_only]

- **WHEN** a canvas-sized edit is previewed at level 0 on a document with no reduced level
- **THEN** the level-0 preview is presented and equals the full composite

#### Scenario: An oversized backdrop is refused [rs_fallback_budget]

- **WHEN** the backdrop plus the scratch canvas would exceed the session's memory budget
- **THEN** no backdrop is cached and the exact path runs

#### Scenario: A soloed mask or live stroke falls back [rs_fallback_state]

- **WHEN** a mask is soloed or a stroke is live
- **THEN** the exact path runs and the displayed frame is the one that state defines

#### Scenario: Content that cannot be reduced falls back [rs_fallback_content]

- **WHEN** a reduced-level preview is requested and the edited stack contains a live type layer, a proxy-less smart object, or a vector fill
- **THEN** the preview and the commit use the exact path and equal the full composite

#### Scenario: An empty visible rectangle is a no-op [rs_fallback_empty]

- **WHEN** the visible document rectangle is empty
- **THEN** no preview is presented and no damage is marked

#### Scenario: No presented level falls back [rs_fallback_no_presented_level]

- **WHEN** the canvas reports no presented level
- **THEN** the exact path runs and the frame equals the full composite
