# Spec Delta

## MODIFIED Requirements

### Requirement: Memory Usage is a Performance preference

The Performance page of Preferences SHALL show a Memory Usage spin box from 5 to
100 percent, default 60, whose value the governor uses at once and which is
saved with the session and restored at start. A slider of the same range SHALL
move with the spin box, and a "Let Pictura Use" line SHALL show the governor's
ceiling in megabytes, or "—" when it has none.

#### Scenario: The default is 60 % [mg_preference_default]

- **WHEN** the application starts without a saved Memory Usage
- **THEN** the governor's Memory Usage is 60 and the spin box shows 60

#### Scenario: A change applies and persists [mg_preference_change]

- **WHEN** the spin box is set to 45 and the session is saved and read back
- **THEN** the governor's Memory Usage is 45 and the restored session holds 45

#### Scenario: The slider and ceiling follow [mg_preference_slider]

- **WHEN** the slider is moved to 30
- **THEN** the spin box and the governor read 30, and the ceiling line shows
  the governor's ceiling in megabytes
