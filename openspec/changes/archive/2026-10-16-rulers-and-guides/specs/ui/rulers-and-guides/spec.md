## ADDED Requirements

### Requirement: Rulers toggle

`View > Rulers` (`Ctrl+R`) SHALL be a checkable command that shows or hides a
horizontal ruler above and a vertical ruler left of every open document
canvas. The rulers SHALL measure from the document's top-left corner in the
ruler unit at the document's resolution, SHALL label distances from 0 without
a sign, SHALL follow the canvas pan and zoom, choosing a labelled major step
of 1, 2, or 5 × 10ⁿ units at least 50 screen pixels apart (eighths for a
1-inch step), and SHALL mark the cursor position. The setting SHALL be global,
off by default, and persisted in the session store.

#### Scenario: Toggle and persist

- **WHEN** `tst_rulers_guides` dispatches `View > Rulers` on a fresh session and saves the session
- **THEN** both rulers of the open document are visible, the stored `rulersVisible` is true, and dispatching again hides them

#### Scenario: Ruler scale follows zoom

- **WHEN** the ruler step is computed for pixels at zoom 1, 0.1, and 8, and for inches at 72 screen pixels per inch
- **THEN** the pixel major steps are 50, 500, and 10, and the inch step is 1 inch in eighths, each at least 50 screen pixels apart

### Requirement: Ruler units

The rulers SHALL default to inches (CS6's default) and a right-click on either
ruler SHALL offer Pixels, Inches, Centimeters, Millimeters, Points (72 per
inch), Picas, and Percent (of the document's width or height). The chosen unit
SHALL apply to both rulers of every document and be persisted.

#### Scenario: Pick a unit

- **WHEN** the test reads the rulers of an 80×60, 72 ppi document and then picks Percent
- **THEN** the rulers start in inches at 72 pixels per unit, then both read percent at 0.8 and 0.6 pixels per unit, and the stored `rulerUnit` is Percent

### Requirement: Units & Rulers preferences

`Edit > Preferences > Units & Rulers` SHALL be a real page (#299) whose Rulers
menu sets the ruler unit, kept in sync with the rulers' context menu, and
whose Point/Pica Size (PostScript 72 or Traditional 72.27 points per inch)
sets the points and picas rulers; both SHALL be persisted. A double-click on a
ruler SHALL open the page. Type, Column Size, and New Document Preset
Resolutions SHALL show CS6's defaults disabled.

#### Scenario: Pick a unit and point size on the page

- **WHEN** the test picks Points on the page, then Traditional, then picks Centimeters from a ruler's context menu, and double-clicks a ruler
- **THEN** both rulers read points at 1 then 72/72.27 pixels per unit, the stored `rulerUnit` / `traditionalPoints` follow, the page shows Centimeters, and the double-click shows the Units & Rulers page

### Requirement: Drag a guide from a ruler

A left-button drag starting on the top ruler SHALL place a horizontal guide,
and one starting on the left ruler a vertical guide, at the whole document
pixel under the release, recording one "New Guide" history state. A release
outside the canvas SHALL place nothing. Placing a guide while guides are
hidden SHALL show them.

#### Scenario: Drop a guide on the canvas

- **WHEN** the test drags from the top ruler to the canvas point over document row 20
- **THEN** the document has one horizontal guide at 20, the newest history state is "New Guide", and undo removes it

#### Scenario: Release off the canvas

- **WHEN** a drag from the left ruler is released back over the ruler
- **THEN** no guide is added and no history state is recorded

### Requirement: Move and delete guides

While guides are shown and unlocked, a left press within 4 screen pixels of a
guide with the Move tool active, or with any tool while `Ctrl` is held, SHALL
drag that guide instead of reaching the tool; the release SHALL record one
"Move Guide" state at the whole pixel under the cursor, or one "Delete Guide"
state when released outside the canvas. Locked or hidden guides SHALL NOT be
picked up.

#### Scenario: Move then delete

- **WHEN** the test drags a vertical guide at 30 to the canvas point over column 45 with the Move tool, then drags it off the canvas
- **THEN** the guide is first at 45 with the newest state "Move Guide", then gone with the newest state "Delete Guide"

#### Scenario: Locked guides stay put

- **WHEN** `View > Lock Guides` is on and the Move tool presses on a guide
- **THEN** the guide does not move and no guide history state is recorded

### Requirement: Guide view commands

`View > Show > Guides` (`Ctrl+;`) and `View > Lock Guides` (`Alt+Ctrl+;`)
SHALL be checkable, global, persisted toggles (shown and unlocked by
default). `View > Clear Guides` SHALL remove every guide of the active
document in one "Clear Guides" state and SHALL be disabled when it has none.
`View > New Guide…` SHALL ask for an orientation and a pixel position and add
that guide in one "New Guide" state.

#### Scenario: Clear and new guide

- **WHEN** the test adds two guides, dispatches Clear Guides, then accepts the New Guide dialog with Vertical at 12
- **THEN** Clear Guides leaves none in one state, it is then disabled, and the dialog adds one vertical guide at 12

### Requirement: Guides preferences

`Edit > Preferences > Guides, Grid, & Slices` SHALL be a real page whose
Guides group offers a colour (Light Blue, Light Red, Green, Medium Blue,
Yellow, Magenta, Cyan, Light Gray, Black, Custom…; default Cyan
`#4AFFFF`) with a swatch, and a style (Lines, Dashed Lines; default Lines).
A change SHALL apply to every open canvas and be persisted. The Smart Guides,
Grid, and Slices groups SHALL show their CS6 defaults disabled.

#### Scenario: Colour and style apply and persist

- **WHEN** the test selects Magenta and Dashed Lines on the page and reloads the session
- **THEN** the canvas draws guides in `#FF4AFF` dashed, and the stored `guideColor` / `guideDashed` are `#ff4aff` / true
