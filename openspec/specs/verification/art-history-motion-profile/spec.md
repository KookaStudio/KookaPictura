# verification/art-history-motion-profile Specification

## Purpose
Makes Art History motion reproducible in the tool profiler: pinned options, a textured workload that really paints, a History Brush reference, and the control op that sets the History Brush source (#1238).

## Requirements

### Requirement: The profiler times Art History motion reproducibly

`scripts/profile-tools.py` SHALL pin the Art History scenarios' options (a
200 px hard tip at 100 % opacity and flow, Tight Short, Area 50, Tolerance 0),
so carry-over from earlier scenarios cannot change the work. It SHALL offer
`art-history` on the new white document, where the source state equals the
layer and no dab paints, and `art-history-textured`, whose setup renders
Clouds, makes that state the History Brush source, and applies Difference
Clouds so the layer has drifted from the source. It SHALL offer
`history-brush-textured`, the same tip and setup with the History Brush, as the
budget reference. Its `--self-check` SHALL run these three scenarios.

#### Scenario: The Art History scenarios run their motion [ahp_scenarios]

- **WHEN** the self-check runs `art-history`, `art-history-textured` and `history-brush-textured` on a 512² document
- **THEN** each times 60 motion events and undoes its stroke

#### Scenario: The textured strokes paint [ahp_textured_paints]

- **WHEN** `art-history-textured` and `history-brush-textured` finish their strokes
- **THEN** the newest history state is the tool's own stroke

### Requirement: The control server sets the History Brush source

The `history` control method SHALL accept `op: "brush_source"` with a state
`index`, setting the History Brush source to that state at the Edit tier. An
index outside the history SHALL be refused with `invalid_param` and SHALL leave
the source unchanged. The `pictura_history` MCP tool SHALL list the op.

#### Scenario: brush_source sets the source state [ahp_brush_source]

- **WHEN** a client sends `history` with `op: "brush_source"` and the current state's index
- **THEN** the History Brush source is that state, the current state is unchanged, and the op's tier is Edit

#### Scenario: An unknown state is refused [ahp_brush_source_invalid]

- **WHEN** the index is past the last state
- **THEN** the reply is `invalid_param` and the source is unchanged
