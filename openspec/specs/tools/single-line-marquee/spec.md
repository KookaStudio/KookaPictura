# tools/single-line-marquee Specification

## Purpose
Select a single document row or column with the marquee tool family.

## Requirements

### Requirement: Single-line marquee discovery
The toolbox SHALL offer Single Row Marquee and Single Column Marquee in the marquee flyout. Both SHALL be selectable from the flyout and through Alt-click cycling of that slot. They SHALL have no default keyboard shortcuts; the default Shift+M cycle SHALL remain Rectangular and Elliptical Marquee.

#### Scenario: Discover the tools [slm_discovery]
- **WHEN** the marquee flyout is opened or its slot is Alt-clicked through its tools
- **THEN** both single-line tools can be selected, have no default shortcut, and Shift+M cycles the rectangle and ellipse

### Requirement: Click selects a band
A click inside the image with Single Row Marquee SHALL select the clicked pixel's entire document row, one pixel high. Single Column Marquee SHALL select the clicked pixel's entire document column, one pixel wide. The selection SHALL use document coordinates and SHALL be clipped to the document. The band SHALL be committed from the click without requiring drag extent. A click in New mode SHALL place the band even inside an existing selection. Each completed selection SHALL be undoable and redoable as one selection change.

#### Scenario: Select a row [slm_row]
- **WHEN** Single Row Marquee clicks pixel (3, 2) of an 8 by 6 document with Feather zero
- **THEN** all and only pixels in row 2 are fully selected

#### Scenario: Select a column [slm_column]
- **WHEN** Single Column Marquee clicks pixel (3, 2) of an 8 by 6 document with Feather zero
- **THEN** all and only pixels in column 3 are fully selected

#### Scenario: Single pixel document [slm_one_pixel]
- **WHEN** either tool clicks a 1 by 1 document with Feather zero
- **THEN** its sole pixel is fully selected

#### Scenario: Click within an existing selection and undo [slm_history]
- **WHEN** New mode clicks inside an existing selection and then Undo and Redo are invoked
- **THEN** the click replaces the selection with the band, Undo restores the prior selection, and Redo restores the band

### Requirement: Single-line options and combination
Both tools SHALL offer New, Add, Subtract and Intersect modes and Feather from 0 to 250 pixels. They SHALL apply feather to the hard band before combining it with the existing selection using the existing selection combine rules. Shift, Alt and Shift+Alt at the click SHALL choose Add, Subtract and Intersect respectively when a selection exists. Neither tool SHALL expose Style or Anti-alias. Both SHALL offer Refine Edge with the same availability and action as other marquee tools. If a nonempty resulting selection has no pixel more than 50% selected, the application SHALL show the warning “No pixels are more than 50% selected” and retain the selection and its undo history.

#### Scenario: Selection mode buttons and modifiers [slm_modes]
- **WHEN** each tool selects a band overlapping an existing selection in each mode, including the corresponding modifier gestures
- **THEN** New replaces, Add unions, Subtract removes, and Intersect retains only overlapping coverage

#### Scenario: Feather softens the band [slm_feather]
- **WHEN** either tool selects a band with nonzero Feather on a document with unselected neighboring pixels
- **THEN** the band is feathered before combination and the resulting mask contains partial coverage

#### Scenario: Feathered full-document band [slm_feather_full]
- **WHEN** either tool selects a band that covers the entire document with nonzero Feather
- **THEN** clamp-to-edge feathering leaves every pixel fully selected

#### Scenario: Feather leaves no visible contour [slm_feather_warning]
- **WHEN** either tool creates a nonempty feathered selection with no pixel more than 50% selected
- **THEN** the warning is shown, the selection remains accepted, and Undo and Redo restore the prior and feathered selections

#### Scenario: Options available [slm_options]
- **WHEN** each single-line tool is active
- **THEN** the options bar offers selection modes, Feather and Refine Edge, but no Style or Anti-alias control

#### Scenario: Refine Edge [slm_refine]
- **WHEN** either tool has a selection and the user activates Refine Edge in its options bar
- **THEN** the same Refine Edge action opens as for the other marquee tools

### Requirement: Selection tool integration
Both single-line tools SHALL participate in selection context menus and selection movement. A drag beginning inside an existing selection SHALL move the selection according to the existing selection-tool gesture, while a click SHALL create the band.

#### Scenario: Move the selection [slm_move]
- **WHEN** either single-line tool drags inside an existing selection without a combine modifier
- **THEN** the existing selection moves according to the selection movement gesture

#### Scenario: Selection context menu [slm_context]
- **WHEN** either single-line tool opens the canvas context menu with a selection
- **THEN** the selection context menu is available
