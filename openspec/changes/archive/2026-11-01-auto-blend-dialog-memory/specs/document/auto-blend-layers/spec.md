## ADDED Requirements

### Requirement: Auto-Blend dialog restores its options

After a run is accepted, the Auto-Blend Layers dialog SHALL open with those options
for the rest of the session. An option unavailable for the current document SHALL
stay disabled without losing the remembered value.

#### Scenario: Options come back [abl_restore]

- **WHEN** the dialog opens with options from an earlier accepted run
- **THEN** the blend method and both checkboxes are restored
