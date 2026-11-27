# Tool framework

## MODIFIED Requirements

### Requirement: Tool registry and active tool

The system SHALL define tools identified by stable `ToolId` values and SHALL track exactly one active tool. The system SHALL refuse to activate a tool that has no engine. Tools with a default shortcut SHALL be selectable by their assigned single-letter shortcut. The system SHALL honour the `Use Shift Key For Tool Switch` preference: when it is on (the default), pressing the letter alone SHALL activate the slot's current member if that member has the shortcut, and `Shift`+letter SHALL cycle to the next implemented member assigned that shortcut; when it is off, the letter alone SHALL cycle. Cycling SHALL wrap deterministically and SHALL skip unimplemented and unassigned members. A group with no implemented member assigned the letter SHALL react to no letter. Single Row and Single Column Marquee SHALL have no default shortcut; when either is the current member, an M-cycle gesture or plain M with the preference on SHALL select Rectangular Marquee. Alt-click cycling SHALL continue to include implemented unassigned members.

#### Scenario: Activating a tool by shortcut [m40_shift]
- **WHEN** the user presses a tool slot's letter
- **THEN** the slot's current member with that shortcut becomes active and no other tool remains active

#### Scenario: Cycling the paint tool slot [m40_shift]
- **WHEN** `Shift+B` is pressed while Brush is active and `Use Shift Key For Tool Switch` is on
- **THEN** Pencil becomes active

#### Scenario: The preference controls whether Shift is required [m40_shift]
- **WHEN** the same letter is pressed with the preference off
- **THEN** the letter alone advances to the next implemented member assigned that shortcut

#### Scenario: A group with no implemented member does nothing [m40_shift]
- **WHEN** the letter of a group whose members are all unimplemented is pressed
- **THEN** the active tool does not change

#### Scenario: Cycle from an unassigned marquee [slm_shortcut_fallback]
- **WHEN** either single-line marquee is current and the user presses the M-cycle gesture, or presses plain M with the Shift preference on
- **THEN** Rectangular Marquee becomes active
