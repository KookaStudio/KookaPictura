# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: A source-depth composite output is exposed`
- TO: `### Requirement: A source-depth composite output is internal`

## MODIFIED Requirements

### Requirement: A source-depth composite output is internal

`pictura_render` SHALL composite a high-depth document at its source depth
internally: `Samples::U16` when the source depth is 16 and `Samples::F32` when it
is 32, in planar RGBA order (straight alpha, clamped to `[0, 1]`), and `None`
when the document has no recorded source depth. The raw source-depth composite
SHALL be internal to the crate. The source-depth output is exposed to the
application only through `refresh_native_composite(doc) -> bool`, which SHALL
remain the application-facing operation that recomputes the source-depth
composite and splices the native color planes into the retained source planes.

#### Scenario: A depth-16 document yields u16 samples

- **WHEN** the source-depth composite runs on a document whose source depth is 16
- **THEN** it produces `Some(Samples::U16(_))` of length `width * height * 4`

#### Scenario: A depth-32 document yields f32 samples

- **WHEN** the source-depth composite runs on a document whose source depth is 32
- **THEN** it produces `Some(Samples::F32(_))`

#### Scenario: An 8-bit document yields nothing

- **WHEN** the source-depth composite runs on an 8-bit or constructed document
- **THEN** it produces `None`

#### Scenario: The application-facing refresh stays exposed

- **WHEN** `refresh_native_composite` is called on a layered document with a
  recorded 16- or 32-bit source depth
- **THEN** it updates the retained source planes and the 8-bit composite and
  returns true
