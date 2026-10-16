# Keyboard Shortcuts

- **Spec ID:** `UI-011`
- **Status:** `Draft`
- **Parity tier:** `Core` (shortcut *editing* and *sets* are Core; some dialog shortcuts cover
  Extended-only features and are `Extended-only` where noted).
- **New in CS6:** `Changed` — CS6 keeps the CS5 shortcut engine and default set but changes
  several defaults and adds shortcuts for new features (Crop-tool overlay keys, Blur Gallery
  `H`/`M`, Liquify `M`/`X`, Adaptive Wide Angle constraint keys, Print `Space`), adds
  `00`/`Shift+00` layer opacity, adds Type-tool dynamic shortcuts, and moves the Type menu.
  CS6 also promotes `Edit > Keyboard Shortcuts` (and `Window > Workspace > Keyboard Shortcuts &
  Menus`) as the shortcut editor, with exportable sets and an HTML **Summarize** report.
- **Depends on:** `02-ui-ux/preferences.md` (`UI-010`), `02-ui-ux/accessibility.md` (`UI-012`),
  `01-architecture/qt6-ui-design.md`, `03-tools/*`, `02-ui-ux/panels/timeline-panel.md`
  (`Enable Timeline Shortcut Keys`).

> Module/type names are design proposals. The shortcut tables below are transcribed from the
> CS6 Help reference and cross-checked against a complete CS6 shortcut sheet. No table rows
> remain unverified; the residual open items are listed under Open questions.

## CS6 behavior

Photoshop CS6 ships a large default shortcut set plus a full shortcut editor. Help states:
"Photoshop lets you view a list of all shortcuts, and edit or create shortcuts. The Keyboard
Shortcuts dialog box serves as a shortcut editor, and includes all commands that support
shortcuts, some of which aren't addressed in the default shortcut set."

### Shortcut editing and saved sets

- **Open the editor:** `Edit > Keyboard Shortcuts`, or
  `Window > Workspace > Keyboard Shortcuts & Menus` → **Keyboard Shortcuts** tab.
- **Set menu:** choose a set; sets are named (a `Photoshop Defaults` set plus user sets).
- **Shortcuts For menu:**
  - **Application Menus** — menu-bar items.
  - **Panel Menus** — panel fly-out menus.
  - **Tools** — toolbox tools.
- **Assign:** select the row, type a new shortcut. If it conflicts, an alert offers **Accept**
  (steals the shortcut), **Undo Changes**, or **Accept and Go To Conflict**.
- **Save / Save Set As / Undo / Use Default / Cancel** buttons manage the current set. Saving
  over `Photoshop Defaults` opens a Save dialog to name a new set.
- **Delete Shortcut** removes one binding; the **Delete icon** removes a whole set.
- **Summarize** exports the current set to an HTML file for display/printing.
- **Workspaces:** "In Photoshop, you can assign keyboard shortcuts to each workspace to navigate
  among them quickly." (`Window > Workspace`).
- **Legacy channel shortcuts:** `Edit > Keyboard Shortcuts` → **Use Legacy Channel Shortcuts**
  switches channel keys between `Ctrl/Command+1` (new default) and `Alt/Option+1` (legacy).
- **Menu customization:** the same dialog's **Menus** tab can set menu-item visibility and
  colors ("Keyboard Shortcuts & Menus").
- `Preferences > General > Use Shift Key For Tool Switch` changes how grouped tool shortcuts
  cycle (see `UI-010`).

### New-in-CS6 feature shortcuts

From the CS6 Help "Key shortcuts for new CS6 features":

**Print:** hold `Space` while choosing `File > Print` to clear print settings.

**Blur Gallery (Field, Iris, Tilt-Shift):** `H` temporarily hides on-canvas UI; `M` temporarily
shows the blur mask.

**Liquify:** `M` loads the last mesh; `X` selects the Mirror tool.

**Crop tool:**

| Result | Windows | Mac OS |
|---|---|---|
| Commit crop | Double-click inside crop box, or `Enter` | Double-click, or `Return` |
| Cancel crop | `Esc` | `Esc` |
| Switch crop-box orientation portrait ↔ landscape | `X` | `X` |
| Front Image (Creative Cloud only) | `I` | `I` |
| Reset crop box | `Backspace`/`Delete` | `Backspace`/`Delete` |
| Cycle overlay options | `O` | `O` |
| Cycle overlay orientation (Triangle, Golden Spiral) | `Shift+O` | `Shift+O` |
| Hide/show cropped area | `/` | `/` |
| Create new crop box | `Shift+drag` | `Shift+drag` |
| Constrain proportions | `Shift+drag` corner handle | `Shift+drag` corner handle |
| Invoke Straighten tool | `Control` | `Command` |
| Prevent crop box from shrinking | `Control+rotate` | `Command+rotate` |
| Restrict to 15° increments | `Shift+rotate` | `Shift+rotate` |
| Restrict to 45° axes | `Shift+drag` image | `Shift+drag` image |
| Temporarily disable snapping to edge | `Control+drag` | `Command+drag` |

**Adaptive Wide Angle — tool shortcuts:** `C` Constraint, `Y` Polygon Constraint, `M` Move,
`H` Hand, `Z` Zoom. **Control shortcuts:** `P` Preview, `W` Show Constraint, `E` Show Mesh,
`T` Correction, `S` Scale, `F` Focal Length, `R` Crop Factor, `A` As Shot. **Hidden:** `L`
toggle transparent matte, `X` temporary zoom, `E` revert last polygon corner.

### Tool shortcuts (default set)

Holding a key temporarily activates a tool; releasing returns to the previous tool. Rows with
multiple tools share one letter — repeatedly press, or `Shift`+letter when **Use Shift Key for
Tool Switch** is on. `Alt/Option`-click a tool cycles its hidden tools (except add-anchor-point,
delete-anchor-point, convert-point).

| Tool (cycle with the letter) | Windows | Mac OS |
|---|---|---|
| Move | `V` | `V` |
| Rectangular Marquee, Elliptical Marquee | `M` | `M` |
| Lasso, Polygonal Lasso, Magnetic Lasso | `L` | `L` |
| Quick Selection, Magic Wand | `W` | `W` |
| Crop, Perspective Crop, Slice, Slice Select | `C` | `C` |
| Eyedropper, Color Sampler, Ruler, Note, Count\*, 3D Material Eyedropper\* | `I` | `I` |
| Spot Healing Brush, Healing Brush, Patch, Content-Aware Move, Red Eye | `J` | `J` |
| Brush, Pencil, Color Replacement, Mixer Brush | `B` | `B` |
| Clone Stamp, Pattern Stamp | `S` | `S` |
| History Brush, Art History Brush | `Y` | `Y` |
| Eraser, Background Eraser, Magic Eraser | `E` | `E` |
| Gradient, Paint Bucket, 3D Material Drop\* | `G` | `G` |
| Dodge, Burn, Sponge | `O` | `O` |
| Pen, Freeform Pen | `P` | `P` |
| Horizontal Type, Vertical Type, Horizontal Type Mask, Vertical Type Mask | `T` | `T` |
| Path Selection, Direct Selection | `A` | `A` |
| Rectangle, Rounded Rectangle, Ellipse, Polygon, Line, Custom Shape | `U` | `U` |
| 3D Object Rotate/Roll/Pan/Slide/Scale* | `K` | `K` |
| 3D Camera Rotate/Roll/Pan/Walk/Zoom* | `N` | `N` |
| Hand | `H` | `H` |
| Rotate View | `R` | `R` |
| Zoom | `Z` | `Z` |

`*` Photoshop Extended only. "Cycle through hidden tools" methods and the shared-letter groups
are also documented in the CS6 Help; the exact per-row `Alt`/`Shift` phrasing is in the full map
below (`Keys for selecting tools`).

### Brush size, hardness, and toolbox keys

| Result | Windows | Mac OS |
|---|---|---|
| Decrease/increase brush size | `[` / `]` | `[` / `]` |
| Decrease/increase brush hardness | `Shift+[` / `Shift+]` | `Shift+[` / `Shift+]` |
| Select previous/next brush preset | `,` / `.` | `,` / `.` |
| Select first/last brush preset | `Shift+,` / `Shift+.` | `Shift+,` / `Shift+.` |
| Restore default foreground/background colours | `D` | `D` |
| Swap foreground/background colours | `X` | `X` |
| Toggle Standard mode / Quick Mask mode | `Q` | `Q` |
| Toggle screen modes (forward / backward) | `F` / `Shift+F` | `F` / `Shift+F` |
| Toggle canvas colour (forward / backward) | `Space+F` / `Space+Shift+F` | `Space+F` / `Space+Shift+F` |

`[` / `]` and `Shift+[` / `Shift+]` come from the CS6 shortcut sheet; the CS6
Help documents the same keys inside individual dialogs (Liquify, Vanishing
Point, Camera Raw) and the Brush panel.

### Dynamic shortcuts (type)

Dynamic shortcuts exist only while entering/editing type or when type is selected, and are
listed in the Character panel menu when available. Examples include Faux Bold/Italic, All Caps,
Small Caps, superscript/subscript, underline/strikethrough, and the standard text-editing keys.
`Type > Font Preview Size` (moved to the Type menu in CS6) is not a dynamic shortcut but a menu
option.

The full default map, by category, follows below. It is the CS6 Help "Default keyboard
shortcuts" section; `†` and `*` footnote the original help (same key used in Liquify; Extended
only). The function-key `Undo/Redo` row and the tool rows for CS6's new tools
(Content-Aware Move, Perspective Crop, 3D Material Eyedropper/Drop) were cross-checked against
the CS6 shortcut sheet.

### Keys for viewing images

| Result | Windows | Mac OS |
|---|---|---|
| Cycle through open documents | Control + Tab | Control + Tab |
| Switch to previous document | Shift + Control + Tab | Shift + Command + ` |
| Close a file in Photoshop and open Bridge | Shift-Control-W | Shift-Command-W |
| Toggle between Standard mode and Quick Mask mode | Q | Q |
| Toggle (forward) between Standard screen mode, Full screen mode with menu bar, and Full screen mode | F | F |
| Toggle (backward) between Standard screen mode, Full screen mode with menu bar, and Full screen mode | Shift + F | Shift + F |
| Toggle (forward) canvas color | Space + F (or right-click canvas background and select color) | Space + F (or Control-click canvas background and select color) |
| Toggle (backward) canvas color | Space + Shift + F | Space + Shift + F |
| Fit image in window | Double-click Hand tool | Double-click Hand tool |
| Magnify 100% | Double-click Zoom tool or Ctrl + 1 | Double-click Zoom tool or Command + 1 |
| Switch to Hand tool (when not in text-edit | Spacebar | Spacebar |
| mode) |  |  |
| Simultaneously pan multiple documents simultaneously with Hand tool | Shift-drag | Shift-drag |
| Switch to Zoom In tool | Control + spacebar | Command + spacebar |
| Switch to Zoom Out tool | Alt + spacebar | Option + spacebar |
| Move Zoom marquee while dragging with the Zoom tool | Spacebar-drag | Spacebar-drag |
| Apply zoom percentage, and keep zoom percentage box active | Shift + Enter in Navigator panel zoom percentage box | Shift + Return in Navigator panel zoom percentage box |
| Zoom in on specified area of an image | Control-drag over preview in Navigator panel | Command-drag over preview in Navigator panel |
| Temporarily zoom into an image | Hold down H and then click in the image and hold down the mouse button | Hold down H and then click in the image and hold down the mouse button |
| Scroll image with Hand tool | Spacebar-drag, or drag view area box in Navigator panel | Spacebar-drag, or drag view area box in Navigator panel |
| Scroll up or down 1 screen | Page Up or Page Down† | Page Up or Page Down† |
| Scroll up or down 10 units | Shift + Page Up or Page Down† | Shift + Page Up or Page Down† |
| Move view to upper-left corner or lower right corner | Home or End | Home or End |
| Toggle layer mask on/off as rubylith (layer mask must be selected) | \ (backslash) | \ (backslash) |

### Keys for Puppet Warp

| Result | Windows | Mac OS |
|---|---|---|
| Cancel completely | Esc | Esc |
| Undo last pin adjustment | Ctrl + Z | Command + Z |
| Select all pins | Ctrl + A | Command + A |
| Deselect all pins | Ctrl + D | Command + D |
| Select multiple pins | Shift-click | Shift-click |
| Move multiple selected pins | Shift-drag | Shift-drag |
| Temporarily hide pins | H | H |

### Keys for Refine Edge

| Result | Windows | Mac OS |
|---|---|---|
| Open the Refine Edge dialog box | Control + Alt + R | Command + Option + R |
| Cycle (forward) through preview modes | F | F |
| Cycle (backward) through preview modes | Shift + F | Shift + F |
| Toggle between original image and selection preview | X | X |
| Toggle between original selection and refined version | P | P |
| Toggle radius preview on and off | J | J |
| Toggle between Refine Radius and Erase Refinements tools | Shift + E | Shift + E |

### Keys for the Filter Gallery

| Result | Windows | Mac OS |
|---|---|---|
| Apply a new filter on top of selected | Alt-click a filter | Option-click a filter |
| Open/close all disclosure triangles | Alt-click a disclosure triangle | Option-click a disclosure triangle |
| Change Cancel button to Default | Control | Command |
| Change Cancel button to Reset | Alt | Option |
| Undo/Redo | Control + Z | Command + Z |
| Step forward | Control + Shift + Z | Command + Shift + Z |
| Step backward | Control + Alt + Z | Command + Option + Z |

### Keys for Liquify

| Result | Windows | Mac OS |
|---|---|---|
| Forward Warp tool | W | W |
| Reconstruct tool | R | R |
| Twirl Clockwise tool | C | C |
| Pucker tool | S | S |
| Bloat tool | B | B |
| Push Left tool | O | O |
| Mirror tool | M | M |
| Turbulence tool | T | T |
| Freeze Mask tool | F | F |
| Thaw Mask tool | D | D |
| Reverse direction for Bloat, Pucker, Push Left, and Mirror tools | Alt + tool | Option + tool |
| Continually sample the distortion | Alt-drag in preview with Reconstruct tool, Displace, Amplitwist, or Affine mode selected | Option-drag in preview with reconstruct tool, Displace, Amplitwist, or Affine mode selected |
| Decrease/increase brush size by 2, or | Down Arrow/Up Arrow in Brush Size, | Down Arrow/Up Arrow in Brush Size, |
| density, pressure, rate, or turbulent jitter by 1 | Density, Pressure, Rate, or Turbulent Jitter text box † | Density, Pressure, Rate, or Turbulent Jittertext box † |
| Decrease/increase brush size by 2, or density, pressure, rate, or turbulent jitter by 1 | Left Arrow/Right Arrow with Brush Size, Density, Pressure, Rate, or Turbulent Jitter slider showing † | Left Arrow/Right Arrow with Brush Size, Density, Pressure, Rate, or Turbulent Jitter slider showing † |
| Cycle through controls on right from top | Tab | Tab |
| Cycle through controls on right from bottom | Shift + Tab | Shift + Tab |
| Change Cancel to Reset | Alt | Option |

### Keys for Vanishing Point

| Result | Windows | Mac OS |
|---|---|---|
| Zoom 2x (temporary) | X | X |
| Zoom in | Control + + (plus) | Command + + (plus) |
| Zoom out | Control + - (hyphen) | Command + - (hyphen) |
| Fit in view | Control + 0 (zero), Double-click Hand tool | Command + 0 (zero), Double-click Hand tool |
| Zoom to center at 100% | Double-click Zoom tool | Double-click Zoom tool |
| Increase brush size (Brush, Stamp tools) | ] | ] |
| Decrease brush size (Brush, Stamp tools) | [ | [ |
| Increase brush hardness (Brush, Stamp tools) | Shift + ] | Shift + ] |
| Decrease brush hardness (Brush, Stamp tools) | Shift + [ | Shift + [ |
| Undo last action | Control + Z | Command + Z |
| Redo last action | Control + Shift + Z | Command + Shift + Z |
| Deselect all | Control + D | Command + D |
| Hide selection and planes | Control + H | Command + H |
| Move selection 1 pixel | Arrow keys | Arrow keys |
| Move selection 10 pixels | Shift + arrow keys | Shift + arrow keys |
| Copy | Control + C | Command + C |
| Paste | Control + V | Command + V |
| Repeat last duplicate and move | Control + Shift + T | Command + Shift + T |
| Create a floating selection from the current selection | Control + Alt + T |  |
| Fill a selection with image under the pointer | Control-drag | Command-drag |
| Create a duplicate of the selection as a floating selection | Control + Alt-drag | Command + Option-drag |
| Constrain selection to a 15° rotation | Alt + Shift to rotate | Option + Shift to rotate |
| Select a plane under another selected plane | Control-click the plane | Command-click the plane |
| Create 90 degree plane off parent plane | Control-drag | Command-drag |
| Delete last node while creating plane | Backspace | Delete |
| Make a full canvas plane, square to the camera | Double-click the Create Plane tool | Double-click the Create Plane tool |
| Show/hide measurements (Photoshop Extended only) | Control + Shift + H | Command + Shift + H |
| Export to a DFX file (Photoshop Extended only) | Control + E | Command + E |
| Export to a 3DS file (Photoshop Extended only) | Control + Shift + E | Command + Shift + E |

### Keys for the Camera Raw dialog box

| Result | Windows | Mac OS |
|---|---|---|
| Zoom tool | Z | Z |
| Hand tool | H | H |
| White Balance tool | I | I |
| Color Sampler tool | S | S |
| Crop tool | C | C |
| Straighten tool | A | A |
| Spot Removal tool | B | B |
| Red Eye Removal tool | E | E |
| Basic panel | Ctrl+Alt+1 | Command+Option+1 |
| Tone Curve panel | Ctrl+Alt+2 | Command+Option+2 |
| Detail panel | Ctrl+Alt+3 | Command+Option+3 |
| HSL/Grayscale panel | Ctrl+Alt+4 | Command+Option+4 |
| Split Toning panel | Ctrl+Alt+5 | Command+Option+5 |
| Lens Corrections panel | Ctrl+Alt+6 | Command+Option+6 |
| Camera Calibration panel | Ctrl+Alt+7 | Command+Option+7 |
| Presets panel | Ctrl+Alt+8 | Command+Option+8 (Mac OS Universal Access zoom shortcut must be disabled in System Preferences) |
| Open Snapshots panel | Ctrl+Alt+9 | Command+Option+9 |
| Parametric Curve Targeted Adjustment tool | Ctrl+Alt+Shift+T | Command+Option+Shift+T |
| Hue Targeted Adjustment tool | Ctrl+Alt+Shift+H | Command+Option+Shift+H |
| Saturation Targeted Adjustment tool | Ctrl+Alt+Shift+S | Command+Option+Shift+S |
| Luminance Targeted Adjustment tool | Ctrl+Alt+Shift+L | Command+Option+Shift+L |
| Grayscale Mix Targeted Adjustment tool | Ctrl+Alt+Shift+G | Command+Option+Shift+G |
| Last-used Targeted Adjustment tool | T | T |
| Adjustment Brush tool | K | K |
| Graduated Filter tool | G | G |
| Increase/decrease brush size | ]/[ | ]/[ |
| Increase/decrease brush feather | Shift + ] / Shift + [ | Shift + ] / Shift + [ |
| Increase/decrease Adjustment Brush tool flow in increments of 10 | = (equal sign) / - (hyphen) | = (equal sign) / - (hyphen) |
| Temporarily switch from Add to Erase mode for the Adjustment Brush tool, or from Erase to Add mode | Alt | Option |
| Increase/decrease temporary Adjustment Brushtool size | Alt + ] / Alt + [ | Option + ] / Option + [ |
| Increase/decrease temporary Adjustment Brushtool feather | Alt + Shift + ] / Alt + Shift + [ | Option + Shift + ] / Option + Shift + [ |
| Increase/decrease temporary Adjustment Brushtool flow in increments of 10 | Alt + = / Alt + - | Option = / Option + - |
| Switch to New mode from Add or Erase mode of the Adjustment Brush tool or the Graduated Filter | N | N |
| Toggle Auto Mask for Adjustment Brush tool | M | M |
| Toggle Show Mask for Adjustment Brush tool | Y | Y |
| Toggle pins for Adjustment Brush tool | V | V |
| Toggle overlay for Graduated Filter, Spot Removaltool, or Red Eye Removal tool. | V | V |
| Rotate image left | L or Ctrl + ] | L or Command + ] |
| Rotate image right | R or Ctrl + [ | R or Command + [ |
| Zoom in | Ctrl + + (plus) | Command + + (plus) |
| Zoom out | Ctrl + - (hyphen) | Command + - (hyphen) |
| Temporarily switch to Zoom In tool (Doesn’t work when Straighten tool is selected. If Crop tool is active, temporarily switches to Straighten tool.) | Ctrl | Command |
| Temporarily switch to Zoom Out tool and change the Open Image button to Open Copy and the Cancel button to Reset. | Alt | Option |
| Toggle preview | P | P |
| Full screen mode | F | F |
| Temporarily activate the White Balance tool and change the Open Image button to Open Object. (Does not work if Crop tool is active.) | Shift | Shift |
| Select multiple points in Curves panel | Click the first point; Shift-click additional points | Click the first point; Shift-click additional points |
| Add point to curve in Curves panel | Control-click in preview | Command-click in preview |
| Move selected point in Curves panel (1 unit) | Arrow keys | Arrow keys |
| Move selected point in Curves panel (10 units) | Shift-arrow | Shift-arrow |
| Open selected images in Camera Raw dialog box from Bridge | Ctrl + R | Command + R |
| Open selected images from Bridge bypassing Camera Raw dialog box | Shift + double-click image | Shift + double-click image |
| Display highlights that will be clipped in Preview | Alt-drag Exposure, Recovery, or Black sliders | Option-drag Exposure, Recovery, or Black sliders |
| Highlight clipping warning | O | O |
| Shadows clipping warning | U | U |
| (Filmstrip mode) Add 1 - 5 star rating | Ctrl+1 - 5 | Command+1 - 5 |
| (Filmstrip mode) Increase/decrease rating | Ctrl+. (period) / Ctrl+, (comma) | Command+. (period) / Command+, (comma) |
| (Filmstrip mode) Add red label | Ctrl+6 | Command+6 |
| (Filmstrip mode) Add yellow label | Ctrl+7 | Command+7 |
| (Filmstrip mode) Add green label | Ctrl+8 | Command+8 |
| (Filmstrip mode) Add blue label | Ctrl+9 | Command+9 |
| (Filmstrip mode) Add purple label | Ctrl+Shift+0 | Command+Shift+0 |
| Camera Raw preferences | Ctrl + K | Command + K |
| Deletes Adobe Camera Raw preferences | Ctrl + Alt (on open) | Option + Shift (on open) |

### Keys for the Black-and-White dialog box

| Result | Windows | Mac OS |
|---|---|---|
| Open the Black-and-White dialog box | Shift + Control + Alt + B | Shift + Command + Option+ B |
| Increase/decrease selected value by 1% | Up Arrow/Down Arrow | Up Arrow/Down Arrow |
| Increase/decrease selected value by 10% | Shift + Up Arrow/Down Arrow | Shift + Up Arrow/Down Arrow |
| Change the values of the closest color slider | Click-drag on the image | Click-drag on the image |

### Keys for Curves

| Result | Windows | Mac OS |
|---|---|---|
| Open the Curves dialog box | Control + M | Command + M |
| Select next point on the curve | + (plus) | + (plus) |
| Select the previous point on the curve | - (minus) | - (minus) |
| Select multiple points on the curve | Shift-click the points | Shift-click the points |
| Deselect a point | Control + D | Command + D |
| To delete a point on the curve | Select a point and press Delete | Select a point and press Delete. |
| Move the selected point 1 unit | Arrow keys | Arrow keys |
| Move the selected point 10 units | Shift + Arrow keys | Shift + Arrow keys |
| Display highlights and shadows that will be clipped | Alt-drag black/white point sliders | Option-drag black/white point sliders |
| Set a point to the composite curve | Control-click the image | Command-click the image |
| Set a point to the channel curves | Shift + Control-click the image | Shift + Command-click the image |
| Toggle grid size | Alt-click the field | Option-click the field |

### Keys for selecting and moving objects

| Result | Windows | Mac OS |
|---|---|---|
| Reposition marquee while selecting ‡ | Any marquee tool (except single column and single row) + spacebar-drag | Any marquee tool (except single column and single row) + spacebar-drag |
| Add to a selection | Any selection tool + Shift-drag | Any selection tool + Shift-drag |
| Subtract from a selection | Any selection tool + Alt-drag | Any selection tool + Option-drag |
| Intersect a selection | Any selection tool (except Quick Selection tool) + Shift-Alt-drag | Any selection tool (except Quick Selection tool) + Shift-Option-drag |
| Constrain marquee to square or circle (if no other selections are active) ‡ | Shift-drag | Shift-drag |
| Draw marquee from center (if no other selections are active) ‡ | Alt-drag | Option-drag |
| Constrain shape and draw marquee from center‡ | Shift + Alt-drag | Shift + Option-drag |
| Switch to Move tool | Control (except when Hand, Slice, Path, Shape, or any Pen tool is selected) | Command (except when Hand, Slice, Path, Shape, or any Pen tool is selected) |
| Switch from Magnetic Lasso tool to Lasso tool | Alt-drag | Option-drag |
| Switch from Magnetic Lasso tool to polygonal Lasso tool | Alt-click | Option-click |
| Apply/cancel an operation of the Magnetic Lasso | Enter/Esc or Control + . (period) | Return/Esc or Command + . (period) |
| Move copy of selection | Move tool + Alt-drag selection ‡ | Move tool + Option-drag selection‡ |
| Move selection area 1 pixel | Any selection + Right Arrow, Left Arrow, | Any selection + Right Arrow, Left Arrow, |
|  | Up Arrow, or Down Arrow† | Up Arrow, or Down Arrow† |
| Move selection 1 pixel | Move tool + Right Arrow, Left Arrow, Up Arrow, or Down Arrow †‡ | Move tool + Right Arrow, Left Arrow, Up Arrow, or Down Arrow †‡ |
| Move layer 1 pixel when nothing selected on layer | Control + Right Arrow, Left Arrow, Up Arrow, or Down Arrow† | Command + Right Arrow, Left Arrow, Up Arrow, or Down Arrow† |
| Increase/decrease detection width | Magnetic Lasso tool + [ or ] | Magnetic Lasso tool + [ or ] |
| Accept cropping or exit cropping | Crop tool + Enter or Esc | Crop tool + Return or Esc |
| Toggle crop shield off and on | / (forward slash) | / (forward slash) |
| Make protractor | Ruler tool + Alt-drag end point | Ruler tool + Option-drag end point |
| Snap guide to ruler ticks (except when View > Snap is unchecked) | Shift-drag guide | Shift-drag guide |
| Convert between horizontal and vertical guide | Alt-drag guide | Option-drag guide |
| ‡ Applies to shape tools |  |  |

### Keys for transforming selections, selection borders, and paths

| Result | Windows | Mac OS |
|---|---|---|
| Transform from center or reflect | Alt | Option |
| Constrain | Shift | Shift |
| Distort | Control | Command |
| Apply | Enter | Return |
| Cancel | Control + . (period) or Esc | Command + . (period) or Esc |
| Free transform with duplicate data | Control + Alt + T | Command + Option + T |
| Transform again with duplicate data | Control + Shift + Alt + T | Command + Shift + Option + T |

### Keys for editing paths

| Result | Windows | Mac OS |
|---|---|---|
| Select multiple anchor points | Direct selection tool + Shift-click | Direct selection tool + Shift-click |
| Select entire path | Direct selection tool + Alt-click | Direct selection tool + Option-click |
| Duplicate a path | Pen (any Pen tool), Path Selection or Direct Selection tool + Control + Alt-drag | Pen (any Pen tool), Path Selection or Direct Selection tool+ Command + Option- drag |
| Switch from Path Selection, Pen, Add Anchor Point, Delete Anchor Point, or Convert Point tools, to Direct Selection tool | Control | Command |
| Switch from Pen tool or Freeform Pen tool to Convert Point tool when pointer is over anchor or direction point | Alt | Option |
| Close path | Magnetic Pen tool-double-click | Magnetic Pen tool-double-click |
| Close path with straight-line segment | Magnetic Pen tool + Alt-double-click | Magnetic Pen tool + Option-double-click |

### Keys for painting

| Result | Windows | Mac OS |
|---|---|---|
| Select foreground color from color picker | Any painting tool + Shift + Alt + right-click and drag | Any painting tool + Control + Option + Command and drag |
| Select foreground color from image with Eyedropper tool | Any painting tool + Alt or any shape tool + Alt (except when Paths option is selected) | Any painting tool + Option or any shape tool + Option (except when Paths option is selected) |
| Select background color | Eyedropper tool + Alt-click | Eyedropper tool + Option-click |
| Color sampler tool | Eyedropper tool + Shift | Eyedropper tool + Shift |
| Deletes color sampler | Color sampler tool + Alt-click | Color sampler tool + Option-click |
| Sets opacity, tolerance, strength, or exposure for painting mode | Any painting or editing tool + number keys (e.g., 0 = 100%, 1 = 10%, 4 then 5 in quick succession = 45%) (When airbrush option is enabled, use Shift + number keys) | Any painting or editing tool + number keys (e.g., 0 = 100%, 1 = 10%, 4 then 5 in quick succession = 45%) (When airbrush option is enabled, use Shift + number keys) |
| Sets flow for painting mode | Any painting or editing tool + Shift + number keys (e.g., 0 = 100%, 1 = 10%, 4 then 5 in quick succession = 45%) (When airbrush option is enabled, omit Shift) | Any painting or editing tool + Shift + number keys (e.g., 0 = 100%, 1 = 10%, 4 then 5 in quick succession = 45%) (When airbrush option is enabled, omit Shift) |
| Mixer Brush changes Mix setting | Alt + Shift + number | Option + Shift + number |
| Mixer Brush changes Wet setting | Number keys | Number key |
| Mixer Brush changes Wet and Mix to zero | 00 | 00 |
| Cycle through blending modes | Shift + + (plus) or – (minus) | Shift + + (plus) or – (minus) |
| Open Fill dialog box on background or standard layer | Backspace or Shift + Backspace | Delete or Shift + Delete |
| Fill with foreground or background color | Alt + Backspace or Control + Backspace† | Option + Delete or Command + Delete† |
| Fill from history | Control + Alt + Backspace† | Command + Option + Delete† |
| Displays Fill dialog box | Shift + Backspace | Shift + Delete |
| Lock transparent pixels on/off | / (forward slash) | / (forward slash) |
| Connects points with a straight line | Any painting tool + Shift-click | Any painting tool + Shift-click |

### Keys for blending modes

| Result | Windows | Mac OS |
|---|---|---|
| Cycle through blending modes | Shift + + (plus) or – (minus) | Shift + + (plus) or – (minus) |
| Normal | Shift + Alt + N | Shift + Option + N |
| Dissolve | Shift + Alt + I | Shift + Option + I |
| Behind (Brush tool only) | Shift + Alt + Q | Shift + Option + Q |
| Clear (Brush tool only) | Shift + Alt + R | Shift + Option + R |
| Darken | Shift + Alt + K | Shift + Option + K |
| Multiply | Shift + Alt + M | Shift + Option + M |
| Color Burn | Shift + Alt + B | Shift + Option + B |
| Linear Burn | Shift + Alt + A | Shift + Option + A |
| Lighten | Shift + Alt + G | Shift + Option + G |
| Screen | Shift + Alt + S | Shift + Option + S |
| Color Dodge | Shift + Alt + D | Shift + Option + D |
| Linear Dodge | Shift + Alt + W | Shift + Option + W |
| Overlay | Shift + Alt + O | Shift + Option + O |
| Soft Light | Shift + Alt + F | Shift + Option + F |
| Hard Light | Shift + Alt + H | Shift + Option + H |
| Vivid Light | Shift + Alt + V | Shift + Option + V |
| Linear Light | Shift + Alt + J | Shift + Option + J |
| Pin Light | Shift + Alt + Z | Shift + Option + Z |
| Hard Mix | Shift + Alt + L | Shift + Option + L |
| Difference | Shift + Alt + E | Shift + Option + E |
| Exclusion | Shift + Alt + X | Shift + Option + X |
| Hue | Shift + Alt + U | Shift + Option + U |
| Saturation | Shift + Alt + T | Shift + Option + T |
| Color | Shift + Alt + C | Shift + Option + C |
| Luminosity | Shift + Alt + Y | Shift + Option + Y |
| Desaturate | Sponge tool + Shift + Alt + D | Sponge tool + Shift + Option + D |
| Saturate | Sponge tool + Shift + Alt + S | Sponge tool + Shift + Option + S |
| Dodge/burn shadows | Dodge tool/Burn tool + Shift + Alt + S | Dodge tool/Burn tool + Shift + Option + S |
| Dodge/burn midtones | Dodge tool/Burn tool + Shift + Alt + M | Dodge tool/Burn tool + Shift + Option + M |
| Dodge/burn highlights | Dodge tool/Burn tool + Shift + Alt + H | Dodge tool/Burn tool + Shift + Option + H |
| Set blending mode to Threshold for bitmap images, Normal for all other images | Shift + Alt + N | Shift + Option + N |

### Keys for selecting and editing text

| Result | Windows | Mac OS |
|---|---|---|
| Move type in image | Control-drag type when Type layer is selected | Command-drag type when Type layer is selected |
| Select 1 character left/right or 1 line down/up, or 1 word left/right | Shift + Left Arrow/Right Arrow or Down Arrow/Up Arrow, or Control + Shift + Left Arrow/Right Arrow | Shift + Left Arrow/Right Arrow or Down Arrow/Up Arrow, or Command + Shift + Left Arrow/Right Arrow |
| Select characters from insertion point to mouse click point | Shift-click | Shift-click |
| Move 1 character left/right, 1 line down/up, or 1 word left/right | Left Arrow/Right Arrow, Down Arrow/Up Arrow, or Control + Left Arrow/Right Arrow | Left Arrow/Right Arrow, Down Arrow/Up Arrow, or Command + Left Arrow/Right Arrow |
| Create a new text layer, when a text layer is selected in the Layers panel | Shift-click | Shift-click |
| Select a word, line, paragraph, or story | Double-click, triple-click, quadruple-click, or quintuple-click | Double-click, triple-click, quadruple-click, or quintuple-click |
| Show/Hide selection on selected type | Control + H | Command + H |
| Display the bounding box for transforming text when editing text, or activate Move tool if cursor is inside the bounding box | Control | Command |
| Scale text within a bounding box when resizing the bounding box | Control-drag a bounding box handle | Command-drag a bounding box handle |
| Move text box while creating text box | Spacebar-drag | Spacebar-drag |

### Keys for formatting type

| Result | Windows | Mac OS |
|---|---|---|
| Align left, center, or right | Horizontal Type tool + Control + Shift + L, C, or R | Horizontal Type tool + Command + Shift + L, C, or R |
| Align top, center, or bottom | Vertical Type tool + Control + Shift + L, C, or R | Vertical Type tool + Command + Shift + L, C, or R |
| Choose 100% horizontal scale | Control + Shift + X | Command + Shift + X |
| Choose 100% vertical scale | Control + Shift + Alt + X | Command + Shift + Option + X |
| Choose Auto leading | Control + Shift + Alt + A | Command + Shift + Option + A |
| Choose 0 for tracking | Control + Shift + Q | Command + Control + Shift + Q |
| Justify paragraph, left aligns last line | Control + Shift + J | Command + Shift + J |
| Justify paragraph, justifies all | Control + Shift + F | Command + Shift + F |
| Toggle paragraph hyphenation on/off | Control + Shift + Alt + H | Command + Control + Shift + Option + H |
| Toggle single/every-line composer on/off | Control + Shift + Alt + T | Command + Shift + Option + T |
| Decrease or increase type size of selected text 2 points or pixels | Control + Shift + < or > † | Command + Shift + < or > † |
| Decrease or increase leading 2 points or pixels | Alt + Down Arrow or Up Arrow†† | Option + Down Arrow or Up Arrow†† |
| Decrease or increase baseline shift 2 points or pixels | Shift + Alt + Down Arrow or Up Arrow†† | Shift + Option + Down Arrow or Up Arrow †† |
| Decrease or increase kerning/tracking 20/1000 ems | Alt + Left Arrow or Right Arrow†† | Option + Left Arrow or Right Arrow†† |

### Keys for slicing and optimizing

| Result | Windows | Mac OS |
|---|---|---|
| Toggle between Slice tool and Slice Selection tool | Control | Command |
| Draw square slice | Shift-drag | Shift-drag |
| Draw from center outward | Alt-drag | Option-drag |
| Draw square slice from center outward | Shift + Alt-drag | Shift + Option-drag |
| Reposition slice while creating slice | Spacebar-drag | Spacebar-drag |
| Open context-sensitive menu | Right-click slice | Control-click slice |

### Keys for using panels

| Result | Windows | Mac OS |
|---|---|---|
| Set options for new items (except for Actions, Animation, Styles, Brushes, Tool Presets, and Layer Comps panels) | Alt-click New button | Option-click New button |
| Delete without confirmation (except for the Brush panel) | Alt-click Delete button | Option-click Delete button |
| Apply value and keep text box active | Shift + Enter | Shift + Return |
| Show/Hide all panels | Tab | Tab |
| Show/Hide all panels except the toolbox and options bar | Shift + Tab | Shift + Tab |
| Highlight options bar | Select tool and press Enter | Select tool and press Return |
| Increase/decrease selected values by 10 | Shift + Up Arrow/Down Arrow | Shift + Up Arrow/Down Arrow |

### Keys for the Actions panel

| Result | Windows | Mac OS |
|---|---|---|
| Turn command on and all others off, or turns all commands on | Alt-click the check mark next to a command. | Option-click the check mark next to a command. |
| Turn current modal control on and toggle all other modal controls | Alt-click | Option-click |
| Change action or action set options | Alt + double-click action or action set | Option + double-click action or action set |
| Display Options dialog box for recorded command | Double-click recorded command | Double-click recorded command |
| Play entire action | Control + double-click an action | Command + double-click an action |
| Collapse/expand all components of an action | Alt-click the triangle | Option-click the triangle |
| Play a command | Control-click the Play button | Command-click the Play button |
| Create new action and begin recording without confirmation | Alt-click the New Action button | Option-click the New Action button |
| Select contiguous items of the same kind | Shift-click the action/command | Shift-click the action/command |
| Select discontiguous items of the same kind | Control-click the action/command | Command-click the action/command |

### Keys for adjustment layers

| Result | Windows | Mac OS |
|---|---|---|
| Choose specific channel for adjustment | Alt + 3 (red), 4 (green), 5 (blue) | Option + 3 (red), 4 (green), 5 (blue) |
| Choose composite channel for adjustment | Alt + 2 | Option + 2 |
| Delete adjustment layer | Delete or Backspace | Delete |
| Define Auto options for Levels or Curves | Alt-click Auto button | Option-click Auto button |

### Keys for the Animation panel in Frames mode

| Result | Windows | Mac OS |
|---|---|---|
| Select/deselect multiple contiguous frames | Shift-click second frame | Shift-click second frame |
| Select/deselect multiple discontiguous frames | Control-click multiple frames | Command-click multiple frames |
| Paste using previous settings without displaying the dialog box | Alt + Paste Frames command from the Panel pop-up menu | Option + Paste Frames command from the Panel pop-up menu |

### Keys for the Animation panel in Timeline Mode (Photoshop Extended)

| Result | Windows | Mac OS |
|---|---|---|
| Start playing the timeline or Animation panel | Spacebar | Spacebar |
| Switch between timecode and frame numbers (current time view) | Alt + click the current-time display in the upper-left corner of the timeline. | Option + click the current-time display in the upper-left corner of the timeline. |
| Expand and collapse list of layers | Alt + click | Option + click on list triangles |
| Jump to the next/previous whole second in timeline | Hold down the Shift key when clicking the Next/Previous Frame buttons (on either side of the Play button). | Hold down the Shift key when clicking the Next/Previous Frame buttons (on either side of the Play button) |
| Increase playback speed | Hold down the Shift key while dragging the current time. | Hold down the Shift key while dragging the current time. |
| Decrease playback speed | Hold down the Control key while dragging the current time. | Hold down the Command key while dragging the current time. |
| Snap an object (keyframe, the current time, layer in point, and so on) to the nearest object in timeline | Shift-drag | Shift-drag |
| Scale (evenly distribute to condensed or extended length) a selected group of multiple keyframes | Alt-drag (first or last keyframe in the selection) | Option-drag (first or last keyframe in the group) |
| Back one frame | Left Arrow or Page Up | Left Arrow or Page Up |
| Forward one frame | Right Arrow or Page Down | Right Arrow or Page Down |
| Back ten frames | Shift + Left Arrow or Shift + Page Up | Shift + Left Arrow or Shift Page Up |
| Forward ten frames | Shift + Right Arrow or Shift + Page Down | Shift + Right Arrow or Shift + Page Down |
| Move to the beginning of the timeline | Home | Home |
| Move to the end of the timeline | End | End |
| Move to the beginning of the work area | Shift + Home | Shift + Home |
| Move to the end of the work area | Shift + End | Shift + End |
| Move to In point of the current layer | Up Arrow | Up Arrow |
| Move to the Out point of the current layer | Down Arrow | Down Arrow |
| Back 1 second | Shift + Up Arrow | Shift + Up Arrow |
| Foward 1 second | Shift + Down Arrow | Shift + Down Arrow |
| Return a rotated document to its original orientation | Esc | Esc |

### Keys for the Brush panel

| Result | Windows | Mac OS |
|---|---|---|
| Delete brush | Alt-click brush | Option-click brush |
| Rename brush | Double-click brush | Double-click brush |
| Change brush size | Alt + right click + drag left or right | Ctrl + Option + drag left or right |
| Decrease/increase brush softness/hardness | Alt + right click + drag up or down | Ctrl + Option + drag up or down |
| Select previous/next brush size | , (comma) or . (period) | , (comma) or . (period) |
| Select first/last brush | Shift + , (comma) or . (period) | Shift + , (comma) or . (period) |
| Display precise cross hair for brushes | Caps Lock or Shift + Caps Lock | Caps Lock |
| Toggle airbrush option | Shift + Alt + P | Shift + Option + P |

### Keys for the Channels panel

| Result | Windows | Mac OS |
|---|---|---|
| Select individual channels | Ctrl + 3 (red), 4 (green), 5 (blue) | Command + 3 (red), 4 (green), 5 (blue) |
| Select composite channel | Ctrl + 2 | Command + 2 |
| Load channel as selection | Control-click channel thumbnail, or Alt + Ctrl + 3 (red), 4 (green), 5 (blue) | Command-click channel thumbnail, or Option + Command + 3 (red), 4 (green), 5 (blue) |
| Add to current selection | Control + Shift-click channel thumbnail. | Command + Shift-click channel thumbnail |
| Subtract from current selection | Control + Alt-click channel thumbnail | Command + Option-click channel thumbnail |
| Intersect with current selection | Control + Shift + Alt-click channel thumbnail | Command + Shift + Option-click channel thumbnail |
| Set options for Save Selection As Channel button | Alt-click Save Selection As Channel button | Option-click Save Selection As Channel button |
| Create a new spot channel | Control-click Create New Channel button | Command-click Create New Channel button |
| Select/deselect multiple color-channel selection | Shift-click color channel | Shift-click color channel |
| Select/deselect alpha channel and show/hide as a rubylith overlay | Shift-click alpha channel | Shift-click alpha channel |
| Display channel options | Double-click alpha or spot channel thumbnail | Double-click alpha or spot channel thumbnail |
| Toggle composite and grayscale mask in Quick Mask mode | ~ (tilde) | ~ (tilde) |

### Keys for the Clone Source panel

| Result | Windows | Mac OS |
|---|---|---|
| Show Clone Source (overlays image) | Alt + Shift | Opt + Shift |
| Nudge Clone Source | Alt + Shift + arrow keys | Opt + Shift + arrow keys |
| Rotate Clone Source | Alt + Shift + < or > | Opt + Shift + < or > |
| Scale (increase or reduce size) Clone Source | Alt + Shift + [ or ] | Opt + Shift + [ or ] |

### Keys for the Color panel

| Result | Windows | Mac OS |
|---|---|---|
| Select background color | Alt-click color in color bar | Option-click color in color bar |
| Display Color Bar menu | Right-click color bar | Control-click color bar |
| Cycle through color choices | Shift-click color bar | Shift-click color bar |

### Keys for the History panel

| Result | Windows | Mac OS |
|---|---|---|
| Create a new snapshot | Alt + New Snapshot | Option + New Snapshot |
| Rename snapshot | Double-click snapshot name | Double-click snapshot name |
| Step forward through image states | Control + Shift + Z | Command + Shift + Z |
| Step backward through image states | Control + Alt + Z | Command + Option + Z |
| Duplicate any image state, except the current state | Alt-click the image state | Option-click the image state |
| Permanently clear history (no Undo) | Alt + Clear History (in History panel pop-up menu) | Option + Clear History (in History panel pop-up menu) |

### Keys for the Info panel

| Result | Windows | Mac OS |
|---|---|---|
| Change color readout modes | Click eyedropper icon | Click eyedropper icon |
| Change measurement units | Click crosshair icon | Click crosshair icon |

### Keys for the Layers panel

| Result | Windows | Mac OS |
|---|---|---|
| Load layer transparency as a selection | Control-click layer thumbnail | Command-click layer thumbnail |
| Add to current selection | Control + Shift-click layer thumbnail. | Command + Shift-click layer thumbnail. |
| Subtract from current selection | Control + Alt-click layer thumbnail. | Command + Option-click layer thumbnail. |
| Intersect with current selection | Control + Shift + Alt-click layer thumbnail. | Command + Shift + Option-click layer thumbnail. |
| Load filter mask as a selection | Control-click filter mask thumbnail | Command-click filter mask thumbnail |
| Load layer mask as a selection | Control-click layer mask thumbnail | Command-click layer mask thumbnail |
| Group layers | Control + G | Command + G |
| Ungroup layers | Control + Shift + G | Command-Shift + G |
| Create/release clipping mask | Control + Alt + G | Command-Option + G |
| Select all layers | Control + Alt + A | Command + Option + A |
| Merge visible layers | Control + Shift + E | Command + Shift + E |
| Create new empty layer with dialog box | Alt-click New Layer button | Option-click New Layer button |
| Create new layer below target layer | Control-click New Layer button | Command-click New Layer button |
| Select top layer | Alt + . (period) | Option + . (period) |
| Select bottom layer | Alt + , (comma) | Option + , (comma) |
| Add to layer selection in Layers panel | Shift + Alt + [ or ] | Shift + Option + [ or ] |
| Select next layer down/up | Alt + [ or ] | Option + [ or ] |
| Move target layer down/up | Control + [ or ] | Command + [ or ] |
| Merge a copy of all visible layers into target layer | Control + Shift + Alt + E | Command + Shift + Option + E |
| Merge layers | Highlight layers you want to merge, then Control + E | Highlight the layers you want to merge, then Command + E |
| Move layer to bottom or top | Control + Shift + [ or ] | Command + Shift + [ or ] |
| Copy current layer to layer below | Alt + Merge Down command from the Panel pop-up menu | Option + Merge Down command from the Panel pop-up menu |
| Merge all visible layers to a new layer above the currently selected layer | Alt + Merge Visible command from the Panel pop-up menu | Option + Merge Visible command from the Panel pop-up menu |
| Show/hide this layer/layer group only or all layers/layer groups | Right-click the eye icon | Control-click the eye icon |
| Show/hide all other currently visible layers | Alt-click the eye icon | Option-click the eye icon |
| Toggle lock transparency for target layer, or last applied lock | / (forward slash) | / (forward slash) |
| Edit layer effect/style, options | Double-click layer effect/style | Double-click layer effect/style |
| Hide layer effect/style | Alt-double-click layer effect/style | Option-double-click layer effect/style |
| Edit layer style | Double-click layer | Double-click layer |
| Disable/enable vector mask | Shift-click vector mask thumbnail | Shift-click vector mask thumbnail |
| Open Layer Mask Display Options dialog box | Double-click layer mask thumbnail | Double-click layer mask thumbnail |
| Toggle layer mask on/off | Shift-click layer mask thumbnail | Shift-click layer mask thumbnail |
| Toggle filter mask on/off | Shift-click filter mask thumbnail | Shift-click filter mask thumbnail |
| Toggle between layer mask/composite image | Alt-click layer mask thumbnail | Option-click layer mask thumbnail |
| Toggle between filter mask/composite image | Alt-click filter mask thumbnail | Option-click filter mask thumbnail |
| Open the layer mask context menu | Right-click layer mask thumbnail | Right-click (Control-click) layer mask thumbnail |
| Delete the active layer mask | Delete or Backspace | Delete or Backspace |
| Toggle rubylith mode for layer mask on/off | \ (backslash), or Shift + Alt-click | \ (backslash), or Shift + Option-click |
| Select all type; temporarily select Type tool | Double-click type layer thumbnail | Double-click type layer thumbnail |
| Create a clipping mask | Alt-click the line dividing two layers | Option-click the line dividing two layers |
| Rename layer | Double-click the layer name | Double-click the layer name |
| Edit filter settings | Double-click the filter effect | Double-click the filter effect |
| Edit the Filter Blending options | Double-click the Filter Blending icon | Double-click the Filter Blending icon |
| Create new layer group below current layer/layer set | Control-click New Group button | Command-click New Group button |
| Create new layer group with dialog box | Alt-click New Group button | Option-click New Group button |
| Create layer mask that hides all/selection | Alt-click Add Layer Mask button | Option-click Add Layer Mask button |
| Create vector mask that reveals all/path area | Control-click Add Layer Mask button | Command-click Add Layer Mask button |
| Create vector mask that hides all or displays path area | Control + Alt-click Add Layer Mask button | Command + Option-click Add Layer Mask button |
| Display layer group properties | Right-click layer group and choose Group Properties, or double-click group | Control-click the layer group and choose Group Properties, or double-click group |
| Select/deselect multiple contiguous layers | Shift-click | Shift-click |
| Select/deselect multiple discontiguous layers | Control-click | Command-click |

### Keys for the Layer Comps panel

| Result | Windows | Mac OS |
|---|---|---|
| Create new layer comp without the New Layer Comp box | Alt-click Create New Layer Comp button | Option-click Create New Layer Comp button |
| Open Layer Comp Options dialog box | Double-click layer comp | Double-click layer comp |
| Rename in-line | Double-click layer comp name | Double-click layer comp name |
| Select/deselect multiple contiguous layer comps | Shift-click | Shift-click |
| Select/deselect multiple discontiguous layer comps | Control-click | Command-click |

### Keys for the Paths panel

| Result | Windows | Mac OS |
|---|---|---|
| Load path as selection | Control-click pathname | Command-click pathname |
| Add path to selection | Control + Shift-click pathname | Command + Shift-click pathname |
| Subtract path from selection | Control + Alt-click pathname | Command + Option-click pathname |
| Retain intersection of path as selection | Control + Shift + Alt-click pathname | Command + Shift + Option-click pathname |
| Hide path | Control + Shift + H | Command + Shift + H |
| Set options for Fill Path with Foreground Colorbutton, Stroke Path with Brush button, Load Path as a Selection button, Make Work Path from Selection button, and Create New Path button | Alt-click button | Option-click button |

### Keys for the Swatches panel

| Result | Windows | Mac OS |
|---|---|---|
| Create new swatch from foreground color | Click in empty area of panel | Click in empty area of panel |
| Set swatch color as background color | Control-click swatch | Command-click swatch |
| Delete swatch | Alt-click swatch | Option-click swatch |

### Keys for 3D tools (Photoshop Extended)

| Result | Windows | Mac OS |
|---|---|---|
| Enable 3D object tools | K | K |
| Enable 3D camera tools | N | N |
| Hide nearest surface | Alt + Ctrl + X | Option + Command + X |
| Show all surfaces | Alt + Shift + Ctrl + X | Option + Shift + Command + X |
| 3D Object Tool | Right-click (Windows) / Control- click (Mac OS) | Alt (Windows) / Option (Mac OS ) |
| Rotate | Changes to Drag tool | Changes to Roll tool |
| Roll | Changes to Slide tool | Changes to Rotate tool |
| Drag | Changes to Orbit tool | Changes to Slide tool |
| Slide | Changes to Roll tool | Changes to Drag tool |
| Scale | Scales on the Z plane | Scales on the Z plane |
| To scale on the Y plane, hold down the Shift key. |  |  |
| Camera Tool | Right-click (Windows) / Control- click (Mac OS) | Alt (Windows) / Option (Mac OS ) |
| Orbit | Changes to Drag tool | Changes to Roll tool |
| Roll | Changes to Slide tool | Changes to Rotate tool |
| Pan | Changes to Orbit tool | Changes to Slide tool |
| Walk | Changes to Roll tool | Changes to Drag tool |

### Keys for measurement (Photoshop Extended)

| Result | Windows | Mac OS |
|---|---|---|
| Record a measurement | Shift + Control + M | Shift + Command + M |
| Deselects all measurements | Control + D | Command + D |
| Selects all measurements | Control + A | Command + A |
| Hide/show all measurements | Shift + Control + H | Shift + Command + H |
| Removes a measurement | Backspace | Delete |
| Nudge the measurement | Arrow keys | Arrow keys |
| Nudge the measurement in increments | Shift + arrow keys | Shift + arrow keys |
| Extend/shorten selected measurement | Ctrl + left/right arrow key | Command + left/right arrow key |
| Extend/shorten selected measurement in increments | Shift + Ctrl + left/right arrow key | Shift +Command + left/right arrow key |
| Rotate selected measurement | Ctrl + up/down arrow key | Command + up/down arrow key |
| Rotate selected measurement in increments | Shift + Ctrl + up/down arrow key | Shift + Command + up/down arrow key |

### Keys for DICOM files (Photoshop Extended)

| Result | Windows | Mac OS |
|---|---|---|
| Zoom tool | Z | Z |
| Hand tool | H | H |
| Window Level tool | W | W |
| Select all frames | Control + A | Command + A |
| Deselect all frames except the current frame | Control + D | Command + D |
| Navigate through frames | Arrow keys | Arrow keys |

### Keys for Extract and Pattern Maker (optional plug-ins)

| Result | Windows | Mac OS |
|---|---|---|
| Fit in window | Control + 0 | Command + 0 |
| Zoom in | Control + + (plus) | Command + + (plus) |
| Zoom out | Control + - (hyphen) | Command + - (hyphen) |
| Cycle through controls on right from top | Tab | Tab |
| Cycle through controls on right from bottom | Shift + Tab | Shift + Tab |
| Temporarily activate Hand tool | Spacebar | Spacebar |
| Change Cancel to Reset | Alt | Option |
| Edge Highlighter tool | B | B |
| Fill tool | G | G |
| Eyedropper tool | I | I |
| Cleanup tool | C | C |
| Edge Touchup tool | T | T |
| Toggle between Edge Highlighter tool and Eraser tool | Alt + Edge Highlighter/Eraser tool | Option + Edge Highlighter/Eraser tool |
| Toggle Smart Highlighting | Control with Edge Highlighter tool selected | Command with Edge Highlighter tool selected |
| Remove current highlight | Alt + Delete | Option + Delete |
| Highlight entire image | Control + Delete | Command + Delete |
| Fill foreground area and preview extraction | Shift-click with Fill tool selected | Shift-click with Fill tool selected |
| Move mask when Edge Touchup tool is selected | Control-drag | Command-drag |
| Add opacity when Cleanup tool is selected | Alt-drag | Option-drag |
| Toggle Show menu options in preview between Original and Extracted | X | X |
| Enable Cleanup and Edge Touchup tools before preview | Shift + X | Shift + X |
| Cycle through Display menu in preview from top to bottom | F | F |
| Cycle through Display menu in preview from bottom to top | Shift + F | Shift + F |
| Decrease/increase brush size by 1 | Down Arrow/Up Arrow in Brush Size text box† | Down Arrow or Up Arrow in Brush Size text box† |
| Decrease/increase brush size by 1 | Left Arrow/Right Arrow with Brush Size Slider showing † | Left Arrow/Right Arrow with Brush Size Slider showing † |
| Set strength of Cleanup or Edge Touch-up tool | 0–9 | 0–9 |
| Delete current selection | Control + D | Command + D |
| Undo a selection move | Control + Z | Command + Z |
| Generate or generate again | Control + G | Command + G |
| Intersect with current selection | Shift + Alt + select | Shift + Option + select |
| Toggle view: original/generated pattern | X | X |
| Go to first tile in Tile History | Home | Home |
| Go to last tile in Tile History | End | End |
| Go to previous tile in Tile History | Left Arrow, Page Up | Left Arrow, Page Up |
| Go to next tile in Tile History | Right Arrow, Page Down | Right Arrow, Page Down |
| Delete current tile from Tile History | Delete | Delete |
| Nudge selection when viewing the original | Right Arrow, Left Arrow, Up Arrow, or Down Arrow | Right Arrow, Left Arrow, Up Arrow, or Down Arrow |
| Increase selection nudging when viewing the original | Shift + Right Arrow, Left Arrow, Up Arrow, or Down Arrow | Shift + Right Arrow, Left Arrow, Up Arrow, or Down Arrow |

### Function keys

| Result | Windows | Mac OS |
|---|---|---|
| Start Help | F1 | Help key |
| Undo/Redo | — (`Ctrl+Z`) | F1 |
| Cut | F2 | F2 |
| Copy | F3 | F3 |
| Paste | F4 | F4 |
| Show/Hide Brush panel | F5 | F5 |
| Show/Hide Color panel | F6 | F6 |
| Show/Hide Layers panel | F7 | F7 |
| Show/Hide Info panel | F8 | F8 |
| Show/Hide Actions panel | F9 | Option + F9 |
| Revert | F12 | F12 |
| Fill | Shift + F5 | Shift + F5 |
| Feather Selection | Shift + F6 | Shift + F6 |
| Inverse Selection | Shift + F7 | Shift + F7 |

The CS6 Help leaves the Windows cell blank for `Undo/Redo`; on Windows `Ctrl+Z`
remains the binding, and on Mac OS `F1` duplicates it because `Help` uses the
dedicated `Help` key. (Verified against the CS6 Help function-key table.)

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Keyboard Shortcuts` | Dialog | `Ctrl/Cmd+Alt/Option+Shift+K` ✓ | Main editor; opens on Application Menus. |
| `Window > Workspace > Keyboard Shortcuts & Menus` | Dialog | — | Same dialog; Keyboard Shortcuts + Menus tabs. |
| `Edit > Preferences > General > Use Shift Key For Tool Switch` | Preference | `Ctrl/Cmd+K` | Changes tool-cycle behavior. |
| `Use Legacy Channel Shortcuts` | Dialog checkbox | — | In the Keyboard Shortcuts dialog. |
| `Summarize` | Dialog button | — | Exports HTML shortcut report. |
| Toolbox tool rows | Tools | letters above | Temporarily activates on hold. |
| Menu items | Application Menus | per default set | Shown in menus and tool tips. |
| Panel fly-outs | Panel Menus | per default set | Editor category. |
| `Animation (Timeline) panel menu > Enable Timeline Shortcut Keys` | Panel pref | — | Gates the Timeline keymap. |
| Context menus | Context menu | right-click / Control-click | Alternate access when a shortcut is unknown. |
| `Type > Font Preview Size` | Menu | — | Moved from Preferences in CS6. |

## Parameters & ranges

A shortcut binding is a structured key chord, not a scalar. Model parameters:

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Modifiers | set | — | Ctrl/Cmd, Alt/Option, Shift | Platform-aware labels. |
| Key | keycode | — | printable keys, F1–F24, arrows, nav, numpad | Some keys reserved by the OS. |
| Context | enum | Application Menus | Application Menus / Panel Menus / Tools | Determines conflict domain. |
| Set name | string | Photoshop Defaults | user-defined | Saved independently per workspace where set. |
| Tool-cycle mode | enum | Shift-gated | Shift-gated / letter-cycles | From General preference. |
| Legacy channel keys | bool | off | on/off | Alt/Option+1 vs Ctrl/Cmd+1. |

## Algorithms & pipeline

- **Resolution order (inferred from CS6 behavior):** active modal/dialog → focused text/type
  editing (dynamic shortcuts) → active tool (transient tool keys) → panel context → application
  menu command. The topmost handler that recognizes the chord consumes it. Adobe does not publish
  this table; treat as a design proposal to reproduce observed behavior.
- **Conflict handling:** assigning an existing chord steals it; `Accept and Go To Conflict`
  navigates to the previous owner. `Use Default` restores per-row defaults; `Undo Changes`
  reverts unsaved edits.
- **Persistence:** shortcut sets are stored in the same version settings folder as preferences
  and are separate from the document; CS6 warns that the reset gesture clears custom shortcuts.
- **Export:** `Summarize` serializes the whole resolved set (Application Menus + Panel Menus +
  Tools) to HTML.
- **Temporary tools:** pressing and holding a tool letter switches to that tool and returns to
  the prior tool on release; this is handled at the input layer, before command dispatch.
- **Modifier probing:** painting/selection tools poll Alt/Option, Shift, Space, Ctrl/Cmd during a
  drag to change mode (add/subtract/intersect, constrain, pan). These are not rebindable.

## Rust module mapping

- `crate::input::keymap::{KeyChord, Modifiers, KeyCode, Context}` — a chord is data
  (`mods + key`), not a string, so it can be compared and serialized deterministically.
- `crate::input::keymap::ShortcutSet` — named set with three maps (application, panel, tools)
  and a `defaults()` constructor seeded from the CS6 tables in this spec.
- `crate::input::keymap::ShortcutResolver` — layered dispatch: modal → dynamic text → transient
  tool → panel → menu. Returns a `CommandId`; unknown chords fall through.
- `crate::input::keymap::ConflictReport` — computed when assigning; powers Accept / Go To
  Conflict.
- `crate::input::keymap::summarize_html(&ShortcutSet) -> String` — parity with CS6 Summarize.
- `crate::input::dispatch::CommandId` — stable enum/id for every menu command, tool, and panel
  action; the shortcut map binds chords to these.
- `crate::prefs` integration: `UseShiftKeyForToolSwitch`, `LegacyChannelShortcuts` come from
  `Preferences` (`UI-010`).
- `crate::input::platform` — maps `Ctrl`/`Cmd`, `Alt`/`Option` labels and reserves OS/desktop
  chords (e.g. AT-SPI/Orca, GNOME shell) so they do not collide (`UI-012`).

Types crossing the boundary: `KeyChord`, `Context`, `CommandId`, `ShortcutSetSnapshot`.

## Qt6 component mapping

- `KeyboardShortcutsDialog` (QDialog) — `Set` combo, `Shortcuts For` combo, `QTreeView` of
  commands with an editable shortcut column (a `QKeySequenceEdit`-style delegate), and
  Save/Save Set As/Undo/Use Default/Cancel/Delete/Summarize buttons.
- `ShortcutTreeModel` (`QAbstractItemModel`) — one row per command; conflict highlighting role.
- `ShortcutEditDelegate` (`QStyledItemDelegate`) — captures a `QKeySequence`, validates conflicts.
- Qt provides `QKeySequence` + `QShortcut`/`QAction` natively; the design uses those rather than a
  custom chord type at the Qt layer, with `cxx-qt` marshalling to `KeyChord` in Rust. Note Qt's
  `QKeySequence` canonicalizes modifiers (Ctrl/Meta/Alt/Shift) and must map Linux `Meta` (Super)
  carefully.
- Menus use `QAction::setShortcut` so the same set drives both display and dispatch.
- QML is *not* used for the editor: native menus, key handling and screen-reader exposure are
  required (`UI-012`).

## Data-model impact

- **Not document data.** Shortcuts/workspaces are application-settings data; they do not enter
  PSD/XMP or the document undo stack.
- **Separate store** from the general preference values; has its own schema version and named
  sets. Each set can be exported/imported (CS6 Summarize HTML is a display/print artifact, not
  necessarily a re-import format).
- **Undo granularity:** dialog-level (Cancel rolls back all unsaved edits); there is no per-edit
  document undo.
- **Command IDs are the stable key**; labels/localization must not key the map, so translated
  menus still resolve.

## Edge cases

- **OS/desktop chord collisions** on Linux (`Super`, workspace switching, Orca screen-reader
  keys): reserve them and surface a conflict rather than silently stealing.
- **Keyboard layout:** non-US layouts and dead keys; bind to key codes + modifiers where
  possible, and document that some printable-key bindings are layout-dependent.
- **Numpad/NumLock** and **F-key** reservations in some window managers.
- **Modal dialogs:** application shortcuts must be suppressed or remapped while a modal owns
  input; only dialog-local shortcuts fire.
- **Type-edit mode:** single-letter tool keys must not fire while text is being entered; dynamic
  shortcuts take precedence.
- **Missing default shortcut** for a command: the command must still be reachable via menus (no
  dead command).
- **Custom shortcut removed:** `Use Default` restores it; otherwise the command is menu-only.
- **Extended-only bindings** (3D, measurement, DICOM, Timeline) must be hidden/absent in
  Standard-edition parity.
- **Localization:** shortcut labels (`Ctrl`/`Cmd`, `Alt`/`Option`) are platform-specific; the
  underlying chord is platform-normalized.

## Parity acceptance criteria

1. Given the default set, every shortcut in the tables above resolves to the documented command
   (spot-check via the Help "Summarize" export compared row-by-row, allowing for the unresolved
   items listed under Open questions).
2. Given `Use Shift Key For Tool Switch` on, `Shift+L` cycles Lasso→Polygonal→Magnetic and `L`
   selects the Lasso; with it off, `L` alone cycles.
3. Given a chord already bound, assigning it to another command offers Accept / Undo / Accept and
   Go To Conflict, and `Use Default` restores the original binding.
4. Given `Summarize`, an HTML file is produced containing all three contexts and the resolved
   chords.
5. Given a saved custom set, relaunching preserves it; the CS6 preferences-reset gesture clears
   custom sets (parity with the CS6 Help warning).
6. Given edit mode in the Type tool, pressing `V` inserts "v" and does not switch to the Move
   tool, while the documented dynamic shortcuts still apply.
7. Given a modal dialog open, application-level shortcuts are suppressed and dialog shortcuts
   work.
8. Given Kooka Pictura on Linux, no default binding collides with a reserved desktop/AT infrastructure
   key without a conflict warning.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (Adobe Photoshop CS6
  Help and tutorials, Feb 2013) — primary. Establishes: "Customizing keyboard shortcuts" (editor,
  sets, Application/Panel/Tools, Save/Save Set As/Undo/Use Default/Cancel, Delete Shortcut, delete
  set, Summarize, legacy channel shortcuts option); "Default keyboard shortcuts" (the full
  category tables transcribed above — tools, viewing, Puppet Warp, Refine Edge, Filter Gallery,
  Liquify, Vanishing Point, Camera Raw, Black-and-White, Curves, selecting/moving, transforming,
  editing paths, painting, blending modes, selecting/editing text, formatting type, slicing,
  panels, Actions, adjustment layers, Animation frames/timeline, Brush, Channels, Clone Source,
  Color, History, Info, Layers, Layer Comps, Paths, Swatches, 3D tools, measurement, DICOM,
  Extract/Pattern Maker, Function keys); "Key shortcuts for new CS6 features" (Print, Blur
  Gallery, Liquify, Crop tool, Adaptive Wide Angle); workspace shortcut assignment; Timeline
  `Enable Timeline Shortcut Keys`; `Type > Font Preview Size` moved from Preferences.
- `https://training-nyc.com/legacy/photoshop_cs6_all_keyboard_shortcuts_sheet.pdf`
  ("Adobe Photoshop CS6 Keyboard Shortcuts", Training NYC) — secondary but complete; confirms the
  function-key table (`Undo/Redo` = `Cmd+Z` on Mac / `F1`), the CS6 tool set (Content-Aware Move
  `J`, Perspective Crop `C`, 3D Material Eyedropper `I`, 3D Material Drop `G`), `[`/`]` size and
  `{`/`}` hardness, brush cycling `,`/`.`/`<`/`>`, `D`/`X`/`Q`/`F`, and
  `Edit > Keyboard Shortcuts` = `Opt+Shift+Cmd+K`.
- `https://web.archive.org/web/20131128145732/http://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html`
  — CS6 Help "Default keyboard shortcuts" as archived; same text as the reference PDF.

## Open questions

- **Exact default shortcuts not extractable cleanly** from the PDF layout (a handful of wrapped
  table rows). *Resolve:* compare against CS6's own `Summarize` HTML from a running install.
- **Dispatch precedence** (modal vs. text vs. tool vs. panel vs. menu) is inferred, not
  documented. *Resolve:* instrument a CS6 install or find an Adobe engineering reference.
- **Set/workspace relationship:** how a shortcut set is bound to a workspace, and whether sets
  are per-workspace or global. *Resolve:* CS6 Help "Workspace basics".
- **Import/export format:** whether `Summarize` HTML is the only export or an importable format
  exists. *Resolve:* CS6 Help / testing.
- **Dynamic shortcut full list:** the Character panel menu lists them only contextually. *Resolve:*
  type-tool testing on CS6.
- **Linux key reservations:** which default CS6 bindings collide with GNOME/KDE/Orca. *Resolve:*
  test matrix on target desktops.
- **Unresolved table cells** (none remain in the transcribed tables; any future capture
  discrepancies) must be confirmed before status `Spec'd`.

