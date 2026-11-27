## ADDED Requirements

### Requirement: Restore damage covers the displayed picture

A history restore SHALL refresh the display from the restored document wherever
the crossed states changed the picture. The damage SHALL include the placed
layer-channel and mask tiles for every crossed delta, even when the document
carries a stored composite. A backward walk that leaves a full anchor SHALL
report an unbounded restore so the display rebuilds from the restored document.

#### Scenario: Multi-stroke jump shows only the earlier strokes [eh_jump_strokes_display]
- **WHEN** several brush strokes have been recorded and the user jumps back to an earlier stroke
- **THEN** the display shows only the strokes recorded at or before that state

#### Scenario: Non-linear branch jump removes the appended stroke [eh_jump_anchor_display]
- **WHEN** a non-linear branch capture appended a stroke and the user jumps back to an earlier state
- **THEN** the display no longer shows the appended stroke
