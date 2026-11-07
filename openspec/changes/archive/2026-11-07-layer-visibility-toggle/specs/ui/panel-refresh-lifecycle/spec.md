## ADDED Requirements

### Requirement: A visibility toggle patches rows without a Layers rebuild

A visibility edit whose influence is bounded SHALL update the affected
Layers-panel rows in place and MUST NOT rebuild the panel's snapshot or its
thumbnails. The edit SHALL still record its history state and the debounced
panel pass SHALL still refresh the other docked panels, History included. An
unbounded visibility edit, which already needs a full recomposite, keeps that
rebuild. A solo visibility edit, which changes every row, SHALL keep the full
rebuild.

#### Scenario: The eye toggle patches the row in place [pl_visibility_eye]

- **WHEN** the user toggles a layer's eye on a document with many layers
- **THEN** the row shows the new visibility and the Layers panel rebuilds no
  snapshot or thumbnail

#### Scenario: A non-panel visibility edit patches the row [pl_visibility_external]

- **WHEN** a menu command, the Properties panel, or the control bridge changes a
  layer's visibility
- **THEN** the affected row shows the new visibility without a snapshot rebuild

#### Scenario: The next edit still rebuilds [pl_visibility_following]

- **WHEN** a non-visibility edit commits after a visibility toggle
- **THEN** the Layers panel rebuilds its snapshot once for that edit

#### Scenario: A solo edit rebuilds every row [pl_visibility_solo]

- **WHEN** a solo visibility edit sets one layer visible and every other node
  hidden
- **THEN** the Layers panel rebuilds its snapshot
