# Spec Delta

## Purpose

Defines how a right-click on the document canvas opens a context menu that fits
the active tool, and what each tool's canvas menu contains.

## ADDED Requirements

### Requirement: Canvas right-click routes to the active tool

A right-click over the document SHALL open the active tool's canvas context
menu, positioned near the pointer. A tool that defines no canvas menu SHALL open
no menu. A right-click on the pasteboard outside the document SHALL open the
canvas-background menu regardless of the active tool.

#### Scenario: A tool with a menu opens it

- **WHEN** the Zoom tool is active and the user right-clicks over the document
- **THEN** the Zoom canvas menu opens at the pointer

#### Scenario: A tool without a menu opens nothing

- **WHEN** the Hand tool is active and the user right-clicks over the document
- **THEN** no context menu opens and the event has no other effect

#### Scenario: The pasteboard opens the background menu

- **WHEN** any tool is active and the user right-clicks on the pasteboard
  outside the document bounds
- **THEN** the canvas-background menu opens

### Requirement: Zoom canvas menu

The Zoom tool's canvas menu SHALL offer Fit on Screen, 100%, 200%, Print Size,
Zoom In, and Zoom Out. Each entry SHALL act on the active document's canvas.
Print Size SHALL set the zoom to 96 divided by the document resolution in ppi.
Every entry SHALL be disabled when no document is open.

#### Scenario: Fit on Screen from the canvas menu

- **WHEN** the Zoom tool is active and the user chooses Fit on Screen
- **THEN** the active canvas fits the document on screen

#### Scenario: The entries disable with no document

- **WHEN** no document is open and the Zoom canvas menu is opened
- **THEN** every entry is disabled

### Requirement: Move canvas menu

When the Move tool is active and Auto Select is off, its canvas menu SHALL list,
topmost first, every visible layer that has a non-transparent pixel under the
pointer; choosing a listed layer SHALL make it the active layer. The menu SHALL
also offer Isolation Mode.

#### Scenario: Layers under the pointer are listed

- **WHEN** the Move tool is active with Auto Select off and the user right-clicks
  over overlapping layer content
- **THEN** the menu lists each layer with a pixel at that point, topmost first

#### Scenario: Choosing a listed layer selects it

- **WHEN** the user chooses a layer from the list
- **THEN** that layer becomes the active layer

#### Scenario: Isolation Mode entry

- **WHEN** the user chooses Isolation Mode from the Move canvas menu
- **THEN** the selected layers are isolated in the Layers panel

### Requirement: Eyedropper canvas menu

The Eyedropper tool's canvas menu SHALL offer Sample Size (Point Sample, 3 by 3,
5 by 5, 11 by 11, 31 by 31, 51 by 51, and 101 by 101) and Sample (All Layers and
Current Layer). Choosing an entry SHALL update the tool option, and the options
bar SHALL reflect the new value.

#### Scenario: Change the sample size

- **WHEN** the user chooses 5 by 5 from the Eyedropper canvas menu
- **THEN** the Eyedropper samples a 5 by 5 average and the options bar shows it

#### Scenario: Change the sample source

- **WHEN** the user chooses Current Layer from the Sample submenu
- **THEN** the Eyedropper samples the current layer only

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

### Requirement: Slice canvas menu

The Slice and Slice Select tools' canvas menu SHALL offer Combine Slices, the
stacking-order entries Bring To Front, Bring Forward, Send Backward, and Send To
Back, and Delete Slice. Combine Slices SHALL be enabled only when more than one
user slice is selected and SHALL NOT combine a layer-based slice.

#### Scenario: Combine two user slices

- **WHEN** two user slices are selected and the user chooses Combine Slices
- **THEN** the two slices become one user slice covering their bounding box

#### Scenario: A layer-based slice cannot combine

- **WHEN** the selection includes a layer-based slice
- **THEN** Combine Slices is disabled

### Requirement: Canvas-background menu

The canvas-background menu SHALL offer the pasteboard colours that `Space+F`
cycles: Black, Dark Gray, Medium Gray, Light Gray, and the custom colour.
Choosing one SHALL set the canvas colour.

#### Scenario: Pick a pasteboard colour

- **WHEN** the user chooses Dark Gray from the canvas-background menu
- **THEN** the canvas colour becomes dark gray

### Requirement: Paint-tool canvas menu

When a paint tool that uses the brush tip is active, a canvas right-click SHALL
open the Brush Preset picker, exposing Size, Hardness, and the preset grid. This
menu is a deliberate product decision, not CS6 parity: the corpus documents the
picker as an options-bar control and the on-canvas size/hardness gesture as the
`Alt+Right-drag` HUD.

#### Scenario: Open the brush picker on the canvas

- **WHEN** the Brush tool is active and the user right-clicks over the document
- **THEN** the Brush Preset picker opens at the pointer

#### Scenario: A preset chosen from the canvas applies

- **WHEN** the user picks a preset in the canvas-opened picker
- **THEN** the active paint tip becomes that preset
