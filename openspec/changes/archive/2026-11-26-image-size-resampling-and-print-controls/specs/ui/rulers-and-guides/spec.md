## MODIFIED Requirements

### Requirement: Units & Rulers preferences

`Edit > Preferences > Units & Rulers` SHALL be a real page (#299) whose Rulers
menu sets the ruler unit, kept in sync with the rulers' context menu, and
whose Point/Pica Size (PostScript 72 or Traditional 72.27 points per inch)
sets the points and picas rulers; both SHALL be persisted. A double-click on a
ruler SHALL open the page. Column Size width and gutter SHALL be editable and
persisted for use by Image Size. Type and New Document Preset Resolutions SHALL
show CS6's defaults disabled.

#### Scenario: Pick a unit and point size on the page

- **WHEN** the test picks Points on the page, then Traditional, then picks Centimeters from a ruler's context menu, and double-clicks a ruler
- **THEN** both rulers read points at 1 then 72/72.27 pixels per unit, the stored `rulerUnit` / `traditionalPoints` follow, the page shows Centimeters, and the double-click shows the Units & Rulers page

#### Scenario: Edit and persist column dimensions [column_preferences]

- **WHEN** the user edits Column Size width and gutter in Units & Rulers, saves the preferences, reopens the page, and uses Columns in Image Size
- **THEN** the saved values are shown and determine the Image Size width, while Type and New Document Preset Resolutions remain disabled
