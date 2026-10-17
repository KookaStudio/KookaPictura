# Test Plan — canvas right-click context menus

One row per spec scenario in `specs/ui/canvas-context-menus/spec.md`. Coverage
names the automated test that fails if the behaviour regresses.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A tool with a menu opens it | Activate the Eyedropper; right-click over the document | The Eyedropper canvas menu is built with Sample Size and Sample submenus | `tst_canvas_context_menus::eyeDropperMenuOpens` |
| A tool without a menu opens nothing | Activate the Hand; right-click over the document | No menu is produced for the active tool | `tst_canvas_context_menus::handOpensNothing` |
| The pasteboard opens the background menu | Right-click outside the document rect with a menu-less tool | The canvas-background colour menu is produced | `tst_canvas_context_menus::pasteboardBackgroundMenu` |
| A tool menu wins on the pasteboard | Zoom active, right-click the workspace behind the document | The Zoom menu opens, not the background menu | `tst_command_tree::toolboxZoomMenu` |
| The menu acts on the clicked document | Two docs open, the second active; Move-right-click the first canvas | The menu lists the first document's layers; choosing one changes the first document | `tst_canvas_context_menus::multiDocumentTargetsClicked` |
| Layers under the pointer are listed | Move (Auto-Select off) over two overlapping layers | All hit layers are returned topmost first | `pictura-app::layers_under_point_lists_hits_topmost_first_skipping_non_hits` + `tst_canvas_context_menus::moveListsLayers` |
| Choosing a listed layer selects it | Choose a layer from the Move menu | That layer becomes active | `tst_canvas_context_menus::moveChooseSelects` |
| Isolation Mode entry | Choose Isolation Mode | `isolationModeRequested` fires and the layers panel isolates | `tst_canvas_context_menus::moveIsolation` |
| Change the sample size | Choose 5 by 5 | Option is 5 and the options bar shows 5 by 5 | `tst_canvas_context_menus::eyeDropperSize` |
| Change the sample source | Choose Current Layer | Option is Current Layer and the options bar shows it | `tst_canvas_context_menus::eyeDropperScope` |
| Toggle Delete Cropped Pixels | Toggle the entry with a crop box present | Option flips and the options-bar checkbox follows | `tst_canvas_context_menus::cropToggleDelete` |
| Reset the crop box | Choose Reset Crop | The box returns to the whole canvas | `tst_canvas_context_menus::cropReset` |
| Pick a pasteboard colour | Choose the second colour | The canvas colour matches that entry | `tst_canvas_context_menus::backgroundColour` |
| Open the brush picker on the canvas | Brush active, right-click over the document | `brushPickerRequested` fires and the picker opens | `tst_canvas_context_menus::brushPickerOpens` |
| A preset chosen from the canvas applies | Pick a preset in the canvas-opened picker | The active paint tip matches the preset | `tst_canvas_context_menus::brushPresetApplies` |
| The brush ring follows the pointer while the picker is open | Open the picker over the canvas, then move the pointer over the document | The picker is a non-grabbing overlay and the canvas reports an active brush outline | `tst_canvas_context_menus::brushPickerKeepsBrushHover` + `tst_canvas_context_menus::brushPickerIsNotACapturingPopup` |
| One press paints and dismisses the picker | Press the pointer on the document while the picker is open | The press is not consumed and the picker closes | `tst_canvas_context_menus::brushPickerDismissesOnOutsidePress` |
