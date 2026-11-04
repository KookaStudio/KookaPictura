## ADDED Requirements

### Requirement: A type layer in a missing family renders with the bundled face

When a type layer's font family is not registered, the renderer SHALL shape,
lay out, and rasterize its text with the bundled Liberation Sans face. When the
resolved face is registered but paints no glyph pixel, the renderer SHALL retry
the layout and paint with the bundled face. The substitution SHALL be
render-time only: the layer's stored font family, its `TySh` data, and the
missing-font detection SHALL keep the original family.

#### Scenario: An unregistered family renders text [mff_unregistered]

- **WHEN** a proxy-less type layer's font family is not registered
- **THEN** its region composites text coverage in the bundled face rather than a
  filled rectangle

#### Scenario: A face that paints nothing retries [mff_retry]

- **WHEN** the face resolved for the family paints no glyph pixel for the text
- **THEN** the renderer lays out and paints the text again with the bundled face

#### Scenario: The stored family is unchanged [mff_stored]

- **WHEN** a type layer in a missing family is composited
- **THEN** its stored family and the missing-font list still name the missing
  family
