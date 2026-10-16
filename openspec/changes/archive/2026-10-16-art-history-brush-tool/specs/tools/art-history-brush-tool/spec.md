## ADDED Requirements

### Requirement: Art History Brush strokes

The Art History Brush SHALL paint stylized strokes whose colours come from the
active layer as it was in the History Brush's source state, recording exactly
one "Art History Brush" history state per stroke that changed pixels. Each dab
SHALL scatter strokes over the Area around it; the Style SHALL set their length,
wander, and curl, with Dab a single mark. Strokes SHALL be random from a seed
that differs between history states, so a seed replays its stroke exactly. The
tool SHALL refuse 16- and 32-bit documents and a source state without a pixel
layer at the active path.

#### Scenario: Painting source colour back as scattered strokes

- **WHEN** the `art_history_brush_tool` self-test drags a 4 px Art History Brush over white paint on a red opening state
- **THEN** one "Art History Brush" state is recorded and more than a hundred pixels turn red again

#### Scenario: Longer styles reach further

- **WHEN** the same stroke is painted with Tight Long and with Dab
- **THEN** the Tight Long marks spread further than the Dab marks

### Requirement: Art History Brush tolerance

The Tolerance SHALL restrict strokes to pixels whose colour differs from the
source by at least the tolerance: at 0 % strokes land anywhere, and at higher
tolerances pixels that still match the source are left alone.

#### Scenario: Full tolerance over matching pixels

- **WHEN** the Art History Brush drags at 100 % Tolerance over red that matches the red source
- **THEN** nothing is painted and no history state is recorded
