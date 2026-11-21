# Spec Delta

## MODIFIED Requirements

### Requirement: Guide view commands

`View > Show > Guides` (`Ctrl+;`) and `View > Lock Guides` (`Alt+Ctrl+;`)
SHALL be checkable, global, persisted toggles (shown and unlocked by
default). `View > Clear Guides` SHALL remove every guide of the active
document in one "Clear Guides" state and SHALL be disabled when it has none.
`View > New Guide…` SHALL ask for an orientation and a pixel position and add
that guide in one "New Guide" state. While `View > Extras` is off, guides SHALL
be hidden without changing the Show checkmark, and turning Extras on SHALL
restore them when the Show toggle is on. Placing a guide while guides are hidden
SHALL enable the Show > Guides toggle and turn Extras on so the new guide is
visible.

#### Scenario: Clear and new guide

- **WHEN** the test adds two guides, dispatches Clear Guides, then accepts the New Guide dialog with Vertical at 12
- **THEN** Clear Guides leaves none in one state, it is then disabled, and the dialog adds one vertical guide at 12

#### Scenario: Extras hides guides [guides_extras]

- **WHEN** guides are shown and the user turns Extras off, then on
- **THEN** the guides disappear and return with the Show checkmark unchanged

#### Scenario: Placing a guide while hidden shows it [guides_place_shows]

- **WHEN** Guides is hidden by the Show toggle or by Extras off and the user drags a new guide from a ruler
- **THEN** the Show toggle is checked, Extras is on, and the new guide is visible

### Requirement: Guides preferences

`Edit > Preferences > Guides, Grid, & Slices` SHALL be a real page whose
Guides group offers a colour (Light Blue, Light Red, Green, Medium Blue,
Yellow, Magenta, Cyan, Light Gray, Black, Custom…; default Cyan
`#4AFFFF`) with a swatch, and a style (Lines, Dashed Lines; default Lines).
A change SHALL apply to every open canvas and be persisted. The Grid group
SHALL be live as specified by `ui/view-menu`. The Smart Guides and Slices
groups SHALL show their CS6 defaults disabled.

#### Scenario: Colour and style apply and persist

- **WHEN** the test selects Magenta and Dashed Lines on the page and reloads the session
- **THEN** the canvas draws guides in `#FF4AFF` dashed, and the stored `guideColor` / `guideDashed` are `#ff4aff` / true

#### Scenario: The Grid group is live [grid_prefs_group]

- **WHEN** the test opens Guides, Grid, & Slices and changes the Grid colour, style, interval and subdivisions
- **THEN** the controls are enabled, the canvas grid updates, and the values persist across a reload
