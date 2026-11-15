# Spec Delta

## ADDED Requirements

### Requirement: Selection Anti-alias persists
The Anti-alias value of the selection options group and of the elliptical
selection options group SHALL be saved and restored with the other selection
tool options. The Magic Wand Anti-alias SHALL be saved and restored with its own
options group, so changing it does not change the lasso tools' value. A missing
Anti-alias field SHALL restore the default of on. Restoring SHALL NOT notify
any change listener.

#### Scenario: The selection Anti-alias values round-trip [top_aa_round_trip]
- **WHEN** the Elliptical Marquee Anti-alias is turned off and the Lasso Anti-alias stays on, the session is saved, and a new window opens
- **THEN** the elliptical group reports off and the selection group reports on

#### Scenario: The Magic Wand keeps its own Anti-alias [top_aa_wand_independent]
- **WHEN** the Magic Wand Anti-alias is turned off while the Lasso Anti-alias stays on, the session is saved, and a new window opens
- **THEN** the Magic Wand reports off and the Lasso reports on
