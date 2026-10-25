# ui/canvas-context-menus Specification

## Purpose
Defines how a right-click on the document canvas opens a context menu that fits
the active tool, and what each tool's canvas menu contains.

## Requirements

### Requirement: Canvas right-click routes to the active tool

A right-click over a document's canvas SHALL open that document's active tool's
context menu, positioned near the pointer and acting on the document that was
clicked rather than another open document. The active tool's menu SHALL take
precedence over the pasteboard fallback wherever it is shown, including over
the pasteboard (the Zoom menu shows there). A tool that defines no canvas menu
SHALL open no menu over the document, and SHALL open the canvas-background menu
when the click is on the pasteboard outside the document.

#### Scenario: A tool with a menu opens it

- **WHEN** the Eyedropper tool is active and the user right-clicks over the
  document
- **THEN** the Eyedropper canvas menu opens at the pointer

#### Scenario: A tool without a menu opens nothing

- **WHEN** the Hand tool is active and the user right-clicks over the document
- **THEN** no context menu opens and the event has no other effect

#### Scenario: The pasteboard opens the background menu

- **WHEN** a tool with no canvas menu is active and the user right-clicks on the
  pasteboard outside the document bounds
- **THEN** the canvas-background menu opens

#### Scenario: A tool menu wins on the pasteboard

- **WHEN** the Zoom tool is active and the user right-clicks on the pasteboard
- **THEN** the Zoom canvas menu opens rather than the canvas-background menu

#### Scenario: The menu acts on the clicked document

- **WHEN** two documents are open, the second is active, and the user
  right-clicks on the first document's canvas with the Move tool
- **THEN** the menu describes the first document and choosing an entry changes
  the first document, not the active one

### Requirement: Move canvas menu

When the Move tool is active and Auto Select is off, its canvas menu SHALL list,
topmost first, every layer the Auto-Select hit test considers a hit under the
pointer — a visible layer with non-zero opacity and a non-transparent pixel at
that point, excluding adjustment layers and honouring any active isolation set.
Choosing a listed layer SHALL make it the active layer. The menu SHALL also
offer Isolation Mode regardless of the Auto Select state.

#### Scenario: Layers under the pointer are listed

- **WHEN** the Move tool is active with Auto Select off and the user right-clicks
  over overlapping layer content
- **THEN** the menu lists each hit layer, topmost first

#### Scenario: Choosing a listed layer selects it

- **WHEN** the user chooses a layer from the list
- **THEN** that layer becomes the active layer

#### Scenario: Isolation Mode entry

- **WHEN** the user chooses Isolation Mode from the Move canvas menu
- **THEN** the selected layers are isolated in the Layers panel

### Requirement: Eyedropper canvas menu

The Eyedropper tool's canvas menu SHALL offer Sample Size (Point Sample, 3 by 3,
5 by 5, 11 by 11, 31 by 31, 51 by 51, and 101 by 101) and Sample (Current Layer,
Current & Below, All Layers, All Layers No Adjustments, and Current & Below No
Adjustments). Choosing an entry SHALL update the tool option, and the options
bar SHALL reflect the new value.

#### Scenario: Change the sample size

- **WHEN** the user chooses 5 by 5 from the Eyedropper canvas menu
- **THEN** the Eyedropper samples a 5 by 5 average and the options bar shows it

#### Scenario: Change the sample source

- **WHEN** the user chooses Current Layer from the Sample submenu
- **THEN** the Eyedropper samples the current layer only and the options bar
  shows it

### Requirement: Crop canvas menu

While a crop box is present, the Crop tool's canvas menu SHALL offer a Delete
Cropped Pixels toggle bound to the current option and a Reset Crop entry that
resets the box to the whole canvas.

#### Scenario: Toggle Delete Cropped Pixels

- **WHEN** the user toggles Delete Cropped Pixels in the context menu
- **THEN** the option flips and the options bar checkbox reflects the new state

#### Scenario: Reset the crop box

- **WHEN** the user chooses Reset Crop
- **THEN** the crop box returns to the whole canvas

### Requirement: Canvas-background menu

The canvas-background menu SHALL offer the pasteboard colours in the same cycle
that `Space+F` steps through; choosing one SHALL set the canvas colour.

#### Scenario: Pick a pasteboard colour

- **WHEN** the user chooses a colour from the canvas-background menu
- **THEN** the canvas colour becomes that colour

### Requirement: Paint-tool canvas menu

When the Brush or Pencil tool is active, a canvas right-click SHALL open the
Brush Preset picker, exposing Size, Hardness, and the preset grid. This menu is
a deliberate product decision, not CS6 parity: the corpus documents the picker
as an options-bar control and the on-canvas size/hardness gesture as the
`Alt+Right-drag` HUD.

#### Scenario: Open the brush picker on the canvas

- **WHEN** the Brush tool is active and the user right-clicks over the document
- **THEN** the Brush Preset picker opens at the pointer

#### Scenario: A preset chosen from the canvas applies

- **WHEN** the user picks a preset in the canvas-opened picker
- **THEN** the active paint tip becomes that preset

### Requirement: The paint-tool picker does not capture the pointer

The Brush Preset picker SHALL be a non-grabbing overlay: it SHALL NOT take a
mouse or keyboard grab, and it SHALL dismiss on an outside pointer press without
consuming that press, and on Escape. While it is open the document canvas SHALL
keep receiving pointer movement so the brush-size cursor ring follows the
pointer, and a single press on the canvas SHALL begin the paint stroke and
dismiss the picker in that same press.

#### Scenario: The brush ring follows the pointer while the picker is open

- **WHEN** the Brush tool is active, the picker is open over the canvas, and the
  user moves the pointer over the document
- **THEN** the brush-size cursor ring follows the pointer

#### Scenario: One press paints and dismisses the picker

- **WHEN** the picker is open and the user presses the pointer on the document
- **THEN** the paint stroke begins with that press and the picker closes

### Requirement: Selection-tool canvas menu

When a selection tool is active — rectangular marquee, elliptical marquee, lasso,
polygonal lasso, magnetic lasso, Magic Wand, or Quick Selection — and the clicked
document has a non-empty selection, a canvas right-click SHALL open a selection
menu at the pointer acting on the clicked document. The menu SHALL offer
Deselect, Inverse, Feather…, and Refine Edge…, reusing the registered
Select commands. With no active selection the tool SHALL open no selection menu,
and a click on the pasteboard outside the document SHALL open the
canvas-background menu instead. Transform Selection SHALL NOT be offered until the
selection-border transform exists.

#### Scenario: A selection opens the menu [sel_menu_opens]

- **WHEN** a selection tool is active with a non-empty selection and the user
  right-clicks over the document
- **THEN** the selection menu opens at the pointer with Deselect, Inverse,
  Feather…, and Refine Edge…

#### Scenario: The entries act on the clicked document [sel_menu_commands]

- **WHEN** the user chooses Deselect, Inverse, Feather…, or Refine Edge… from
  the selection menu
- **THEN** that Select command runs on the document that was clicked

#### Scenario: No selection opens the background menu on the pasteboard [sel_menu_no_selection]

- **WHEN** a selection tool is active with no selection and the user right-clicks
  on the pasteboard outside the document
- **THEN** no selection menu opens and the canvas-background menu opens instead
