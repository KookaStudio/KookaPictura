## ADDED Requirements

### Requirement: Stack Images colour and focus edges

With Seamless on, Stack Images SHALL take only broad brightness from the lowest
covering layer; colour SHALL stay with the layer chosen for each area. Focus
selection SHALL stop at edges of the layer being scored, so a subject's detail does
not select the flat background beside it. This is an approximation tuned against
observed Photoshop output.

#### Scenario: The subject keeps its colour [abl_subject_colour]

- **WHEN** a red detailed subject is stacked over a blue base with Seamless on
- **THEN** the merged subject stays red

#### Scenario: No halo beside the subject [abl_focus_edge]

- **WHEN** a detailed subject on a flat background is stacked over a flat base
- **THEN** the mask all but hides the background right beside the subject
