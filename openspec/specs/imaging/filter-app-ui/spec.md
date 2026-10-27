# filter-app-ui Specification

## Purpose
The PictureView filter command, filter-kind mapping and defaults, and the filter dock controls.

## Requirements

### Requirement: PictureView filter command

`PictureView` MUST expose `apply_filter(kind: &QString) -> bool`. It SHALL apply the mapped filter to the topmost pixel layer — the last layer in the bottom-first `doc.layers` for which `adjustment.is_none() && !is_group` — using the active selection as the mask, then recomposite and emit `changed`. With no active selection it MUST apply over the full frame. It MUST return false when there is no document or when there is no pixel layer.

#### Scenario: Applies to the topmost pixel layer

- **WHEN** a document has several pixel layers and `apply_filter` is called
- **THEN** the last bottom-first pixel layer (skipping adjustment and group layers) is the one filtered

#### Scenario: A selection confines the change

- **WHEN** an active selection covers part of the document and `apply_filter` is called
- **THEN** selected pixels change and unselected pixels are unchanged after recompositing

#### Scenario: No selection applies full-frame

- **WHEN** `apply_filter` is called with no active selection
- **THEN** the filter applies to the whole layer rect

#### Scenario: No document or no pixel layer returns false

- **WHEN** `apply_filter` is called with no document, or with a document that has no pixel layer
- **THEN** it returns false and the document is unchanged

### Requirement: Filter-kind mapping and defaults

The command SHALL map each recognized `kind` to a concrete `pictura_filters::Filter` with fixed defaults. The mapping SHALL include at least: `gaussian-blur` → `GaussianBlur { radius: 5.0 }`, `box-blur` → `BoxBlur { radius: 3 }`, `motion-blur` → `MotionBlur { angle: 0.0, distance: 15 }`, `median` → `Median { radius: 2 }`, `despeckle` → `Despeckle`, `sharpen` → `Sharpen`, `sharpen-more` → `SharpenMore`, `unsharp-mask` → `UnsharpMask { amount: 150.0, radius: 1.0, threshold: 0 }`, `add-noise` → `AddNoise { amount: 25.0, distribution: Uniform, monochromatic: false, seed: 1 }`, `dust-and-scratches` → `DustAndScratches { radius: 1, threshold: 0 }`, `extrude` → `Extrude { kind: Blocks, size: 30, depth: 30.0, level_based: true, solid_front: false, mask_incomplete: false }`, `tiles` → `Tiles { count: 10, offset: 10, fill: BackgroundColor, foreground: [0, 0, 0], background: [255, 255, 255] }`, `trace-contour` → `TraceContour { level: 128, edge: Lower }`, `wind` → `Wind { method: Wind, from_right: true }`, and `smart-sharpen` → `SmartSharpen { amount: 100.0, radius: 1.0, reduce_noise: 0.0, remove: GaussianBlur, angle: 0.0 }`. The complete set of recognised kinds, with their canonical slot order, is the filter parameter descriptor; a kind's canonical slot count is the sum of its descriptor's control slot counts. An unrecognized `kind` SHALL map to no filter and the command MUST return false without changing the document.

The command SHALL additionally accept an ordered list of floating-point parameter values for a recognized `kind`, building the same `pictura_filters::Filter` with those values in the kind's canonical slot order. The existing default mapping SHALL be equivalent to supplying the documented defaults for that kind. A kind with a fixed slot list MUST refuse a non-empty list whose length differs from its canonical slot count. The kinds with a documented variable list — `lighting-effects` (9 rig slots plus 13 per light, 1 to 16 lights) and the Blur Gallery kinds (a pin, path or spin count followed by fixed-stride records, then the shared 13-slot effects tail; `blur-gallery` additionally carries its per-effect counts, motion section and flash duration) — MUST accept exactly the lists those kinds document and refuse any other non-empty list. Every kind accepts the canonical-length list built from its descriptor's defaults.

#### Scenario: Each known kind maps to its default filter

- **WHEN** the command is called with each supported kind and no parameters
- **THEN** the corresponding `Filter` with the listed defaults is applied

#### Scenario: A parameter list overrides the defaults

- **WHEN** the command is called with `gaussian-blur` and a slot list of one value
- **THEN** `GaussianBlur` is built with that radius instead of the default `5.0`

#### Scenario: A wrong-arity parameter list is refused

- **WHEN** the command is called with a known kind and a parameter list of the wrong length
- **THEN** it returns false and no layer is modified

#### Scenario: Unknown kind is refused

- **WHEN** the command is called with an unrecognized kind
- **THEN** it returns false and no layer is modified

#### Scenario: The canonical slot count is the descriptor's [fpd_arity]

- **WHEN** a recognised kind is called with its descriptor's default vector, then with one more zero and — when that count is greater than one — one fewer
- **THEN** the default vector maps to a filter, and the longer and shorter lists are refused without changing the document

#### Scenario: The variable-list kinds accept their documented lists [fpd_variable_lists]

- **WHEN** `lighting-effects` is mapped with a rig of 1 to 16 lights, and each Blur Gallery kind with one of its documented pin, path or spin lists (or, for `blur-gallery`, its combined list)
- **THEN** each maps to a filter, and a non-empty list that does not decode as one of those shapes is refused

### Requirement: Filter dock controls

The Qt dock MUST provide a filter combo box listing the supported kinds and an "Apply Filter" button that calls `apply_filter` with the combo's current kind. The image MUST refresh when the command succeeds.

#### Scenario: The Apply Filter button runs the selected filter

- **WHEN** a kind is chosen in the combo and "Apply Filter" is clicked
- **THEN** `apply_filter` runs with that kind and the displayed image refreshes

### Requirement: Filter confinement self-test

The headless `--self-test` MUST load a layered PSD, select a proper subset of the document, apply a filter to the topmost pixel layer confined by that selection, and assert that pixels outside the selection are unchanged while the selected region changed. It MUST exit non-zero on failure and MUST clean up before the remaining checks.

#### Scenario: Filtering is confined to the selection

- **WHEN** `--self-test` runs against a layered PSD
- **THEN** the selected region changes, pixels outside it are unchanged, and the run exits 0

#### Scenario: A failure exits non-zero

- **WHEN** the filtered result differs outside the selection or does not change inside it
- **THEN** the self-test exits non-zero

### Requirement: Artistic filter family exposure

The `Filter` menu SHALL contain an `Artistic` submenu listing all fifteen CS6 Artistic filters in CS6 order (`Colored Pencil`, `Cutout`, `Dry Brush`, `Film Grain`, `Fresco`, `Neon Glow`, `Paint Daubs`, `Palette Knife`, `Plastic Wrap`, `Poster Edges`, `Rough Pastels`, `Smudge Stick`, `Sponge`, `Underpainting`, `Watercolor`). Each entry SHALL open its parameter dialog, map to the corresponding engine kind, and be enabled exactly when the active layer is a filter target; a filter with no parameters SHALL apply directly.

#### Scenario: Every Artistic entry opens its dialog and applies

- **WHEN** each `Filter ▸ Artistic` entry is chosen on a document with an editable pixel layer
- **THEN** it maps to its engine kind and either opens the parameter dialog or applies directly, and the layer's colour planes change

#### Scenario: Disabled without a filter target

- **WHEN** no editable pixel layer is active
- **THEN** the Artistic entries are disabled

### Requirement: New filter-kind mapping

The filter mapping SHALL additionally recognise `lighting-effects`, `diffuse` (one mode slot, default Normal), and `glowing-edges` (width, brightness, smoothness; defaults 2, 6, 1), each with an empty slot list producing its defaults. `lighting-effects` SHALL take 9 rig slots (colorize r, g, b, exposure, gloss, metallic, ambience, texture, height) followed by 13 slots per light (type, visible, colour r, g, b, intensity, hotspot, centre x, centre y, angle, size, width, elevation). It SHALL accept any whole number of lights from 1 to 16, report the one-light total (22) as its arity, and default to CS6's Default style. `diffuse` and `glowing-edges` SHALL refuse a non-empty list of the wrong length, and `lighting-effects` SHALL refuse a list that is not a whole rig of 1 to 16 lights. These kinds SHALL drive `Filter ▸ Render ▸ Lighting Effects`, `Filter ▸ Stylize ▸ Diffuse`, and `Filter ▸ Stylize ▸ Glowing Edges`.

#### Scenario: The new kinds resolve and default

- **WHEN** each new kind is resolved with an empty slot list
- **THEN** the corresponding `Filter` with the listed defaults is produced

#### Scenario: Wrong arity is refused

- **WHEN** a new kind is resolved with a slot list of the wrong length, or `lighting-effects` with a partial light or 17 lights
- **THEN** no filter is produced and the command returns false without changing the document

#### Scenario: A multi-light rig maps

- **WHEN** `lighting-effects` is resolved with the rig slots and two lights, a red Spot and a hidden Infinite light
- **THEN** the `Lighting` carries the rig's colorize, texture, and height and both lights in order, with their type, colour, centre, width, angle, elevation, and visibility

### Requirement: Convert for Smart Filters command

`Filter ▸ Convert for Smart Filters` SHALL convert the active raster layer into
a smart object so that the layer's content is carried as an embedded source,
while leaving the layer's pixels unchanged. It SHALL be enabled only when the
active layer can be converted to a smart object, which now includes the
Background layer; converting a Background SHALL unlock it first. On success it
SHALL recomposite and record one history state named `Convert for Smart Filters`.

#### Scenario: Converting preserves the pixels and records one state

- **WHEN** `Convert for Smart Filters` is invoked on a convertible raster layer
- **THEN** the layer becomes a smart object with the same composited pixels and exactly one history state is recorded

#### Scenario: A Background converts and is unlocked

- **WHEN** `Convert for Smart Filters` is invoked on the Background layer
- **THEN** the layer becomes a smart object, is no longer the Background, its pixels are unchanged, and exactly one history state is recorded

#### Scenario: Refused when not convertible

- **WHEN** `Convert for Smart Filters` is invoked on a group layer or an adjustment layer
- **THEN** the operation returns `false`, the history count is unchanged, and the document is unchanged

### Requirement: Parameter slider activates its value box

A filter parameter row that carries both a slider and a value box SHALL focus the
value box when the slider is clicked, so Up/Down (and typing) adjust the same
parameter without a separate click into the box.

#### Scenario: Clicking the slider focuses the box [fa_slider_focus]

- **WHEN** a parameter slider is clicked
- **THEN** its value box takes focus, so a following Up/Down changes the
  parameter

### Requirement: Liquify workspace

`Filter ▸ Liquify…` (Shift+Ctrl+X) SHALL be enabled whenever filters are and SHALL open the Liquify workspace over the active layer, titled `Liquify (<document> @ <zoom>%)`, at 100 % zoom. A right click with the Zoom tool SHALL offer 6, 12, 25, 33, 50, 66, 100, 200, 300, 400, 600, 800, 1000, 1200 and 1600 %, Actual Pixels, Fit in View and Fit on Screen. A magnified preview SHALL interpolate the distorted layer at screen resolution rather than enlarge its pixels. Basic mode SHALL show Forward Warp, Reconstruct, Pucker, Bloat, Push Left, Hand and Zoom with Size (default 100, up to 15,000) and Pressure (default 100); Advanced Mode SHALL add Twirl Clockwise, Mirror, Turbulence, Freeze Mask and Thaw Mask, Density (50), Rate (80) and Turbulent Jitter (50), Reconstruct…, the Mask Options and View Options: Show Image (on), Show Mesh (off) with Mesh Size (Small, Medium, Large; Medium) and Mesh Color (Red, Yellow, Green, Cyan, Blue, Magenta, Gray; Gray), Show Mask with Mask Color (Red), and Show Backdrop (off) with Use (All Layers or a pixel layer), Mode (In Front, Behind, Blend; In Front) and Opacity (0–100; 50). Each option's controls SHALL be disabled until its checkbox is on. The mesh SHALL be drawn where the distortion places the grid, and the backdrop SHALL be drawn undistorted in front of or behind the distorted layer at Opacity. Brush strokes SHALL preview in the workspace without changing the document. Cancel SHALL leave the document and history untouched; OK with no distortion SHALL record nothing; OK with a distortion SHALL commit one filter history state. The `liquify` kind SHALL take one slot naming the kept mesh; an empty list SHALL be an identity mesh and an id no longer kept SHALL be refused. Load Last Mesh SHALL reload the mesh of the last OK, and Last Filter Settings SHALL reopen the workspace with it.

#### Scenario: Basic mode shows CS6's short column

- **WHEN** the workspace opens
- **THEN** only Forward Warp, Reconstruct, Pucker, Bloat, Push Left, Hand and Zoom are shown, Size and Pressure read 100, and Advanced Mode reveals the remaining tools and options

#### Scenario: The workspace opens at 100% and the Zoom menu sets levels

- **WHEN** the workspace opens and 400% is chosen from the Zoom tool's menu
- **THEN** it opened at 100% and now shows 400%, and across a stripe edge the magnified view blends rather than repeating pixels in blocks

#### Scenario: The mesh follows the warp and the backdrop shows in front or behind

- **WHEN** Show Mesh is on and a Forward Warp drag is made, then Show Backdrop is turned on with Opacity 100
- **THEN** the drawn mesh changes with the drag; In Front shows the undistorted layers over the warp, Behind leaves the opaque warped layer showing, and Opacity 0 shows the full effect

#### Scenario: A drag previews, Cancel discards, OK commits

- **WHEN** a Forward Warp drag is made and Cancel is pressed
- **THEN** the workspace preview changed but the document and history did not; repeating the drag and pressing OK adds one history state with the `liquify` kind and a nonzero mesh id

#### Scenario: Load Last Mesh reapplies the committed mesh

- **WHEN** the workspace opens after a committed Liquify and Load Last Mesh is pressed
- **THEN** the preview shows that distortion again

### Requirement: Shape Blur dialog

`Filter ▸ Blur ▸ Shape Blur…` SHALL be enabled whenever filters are and SHALL sit after Box Blur in the Blur submenu. It SHALL open the filter dialog with `Radius:` (`5..=1000` pixels, default 10) and a `Shape:` grid holding every Custom Shape in the catalogue's order, each with its name as a tooltip, the first selected by default. The `shape-blur` kind SHALL take 2 slots in order: Radius, then the shape's index; an empty list SHALL be radius 10 with the first shape, an index past the catalogue SHALL clamp to the last shape, and any other length SHALL be refused. Changing the radius or the shape SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Shape Blur`, and `Filter ▸ Last Filter` SHALL re-apply the same shape and radius.

#### Scenario: The dialog opens on radius 10 and the first shape

- **WHEN** the Shape Blur dialog opens on a document
- **THEN** its values are `10, 0`, the radius box ranges `5..=1000`, and the grid lists every Custom Shape with the Heart second

#### Scenario: Picking a shape fills the second slot

- **WHEN** the third shape in the grid is selected
- **THEN** slot 1 of the dialog's values is 2

#### Scenario: Picking previews, commits, and repeats

- **WHEN** the dialog is opened from the menu, the Heart and radius 6 are chosen, and OK is pressed
- **THEN** the canvas changes before OK, one `Filter: Shape Blur` history state is added, and `Filter ▸ Last Filter`, labelled `Shape Blur`, blurs the picture again

### Requirement: Smart Blur dialog

`Filter ▸ Blur ▸ Smart Blur…` SHALL be enabled whenever filters are. It SHALL open the filter dialog with `Radius:` and `Threshold:` (`0.1..=100`, one decimal, defaults 3.0 and 25.0), `Quality:` (Low, Medium, High; default Low) and `Mode:` (Normal, Edge Only, Overlay Edge; default Normal). The `smart-blur` kind SHALL take 4 slots in that order; an empty list SHALL be the defaults, a radius or threshold outside `0.1..=100` SHALL clamp into it, and any other length SHALL be refused. Changing a control SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Smart Blur`, and `Filter ▸ Last Filter` SHALL re-apply the same values.

#### Scenario: The dialog opens on CS6's controls

- **WHEN** the Smart Blur dialog opens on a document
- **THEN** its values are `3, 25, 0, 0`, both value boxes range `0.1..=100`, and the menus list Low, Medium, High and Normal, Edge Only, Overlay Edge

#### Scenario: Edge Only previews, commits, and repeats

- **WHEN** Mode is set to Edge Only on a picture of two flat halves and OK is pressed
- **THEN** the canvas turns black-and-white before OK, one `Filter: Smart Blur` state is added with white along the split, and `Filter ▸ Last Filter`, labelled `Smart Blur`, applies Edge Only again

### Requirement: Custom dialog

`Filter ▸ Other ▸ Custom…` SHALL be enabled whenever filters are and SHALL sit first in the Other submenu, ahead of High Pass. It SHALL open the filter dialog with a 5×5 grid of integer weight cells, each `-999..=999`, then `Scale:` (`1..=9999`) and `Offset:` (`-9999..=9999`) as value boxes with no slider. The dialog SHALL open on the pass-through kernel: centre cell 1, every other cell 0, Scale 1, Offset 0. The `custom` kind SHALL take 27 slots in order: the 25 cells left to right, top to bottom, then Scale, then Offset; an empty list SHALL be the pass-through kernel and any other length SHALL be refused. Editing a cell or box SHALL preview on the canvas as other filter dialogs do, OK SHALL commit one history state labelled `Filter: Custom`, and `Filter ▸ Last Filter` SHALL re-apply the same kernel.

#### Scenario: The dialog opens on the pass-through kernel

- **WHEN** the Custom dialog opens on a document
- **THEN** it shows 25 cells ranged `-999..=999`, two value boxes and no slider, and its values are 12 zeros, a 1, 12 zeros, then `1, 0`

#### Scenario: The grid reads row by row

- **WHEN** the first cell of the grid's second row is set to -3
- **THEN** slot 5 of the dialog's values is -3

#### Scenario: Inverting through the grid previews, commits, and repeats

- **WHEN** the dialog is opened from the menu, the centre cell is set to -1 and Offset to 255, and OK is pressed
- **THEN** the canvas changes before OK, one `Filter: Custom` history state is added, every channel reads `255 - v`, and `Filter ▸ Last Filter`, labelled `Custom`, inverts the picture back

### Requirement: Filter parameter dialog

The Qt `Filter` menu SHALL open a parameter dialog for every recognized kind that takes parameters. The dialog MUST show a preview thumbnail of the filtered result with its own zoom controls (magnifier icons and a percentage readout), a Preview checkbox, and one control per parameter of the kind. The thumbnail SHALL show the document section the canvas is currently viewing at that zoom, seeded from the canvas zoom, rather than the whole image scaled down. Radial Blur MUST omit the thumbnail and show a draggable Blur Center instead; the centre is two engine slots (x, y, normalized 0..1) that move the blur origin. Lens Flare MUST show a whole-image placement control for its center. Kinds with no parameters (Average, Blur, Blur More, Sharpen) MUST apply without opening a dialog.

#### Scenario: A parameterised filter opens its dialog

- **WHEN** the user chooses `Filter > Blur > Gaussian Blur…`
- **THEN** a dialog opens with a Radius control and a preview thumbnail

#### Scenario: The preview zoom is independent of the document

- **WHEN** the user clicks the dialog's zoom-in or zoom-out control
- **THEN** the percentage readout steps and only the thumbnail scales; the document pixels and the document zoom are unchanged

#### Scenario: The preview shows the current canvas section

- **WHEN** a dialog opens while the canvas is viewing a particular section at a particular zoom
- **THEN** the thumbnail shows that document section at the seeded zoom, not the whole image shrunk to fit

#### Scenario: Radial Blur shows the Blur Center instead of a thumbnail

- **WHEN** the user opens `Filter > Blur > Radial Blur…`
- **THEN** the dialog shows Amount, Blur Method, Quality, and a draggable Blur Center, and no preview thumbnail; the centre's x and y are slots 3 and 4 of the `radial-blur` list

#### Scenario: A parameterless filter applies directly

- **WHEN** the user chooses `Filter > Sharpen > Sharpen`
- **THEN** the filter applies immediately with no dialog

### Requirement: Live non-committing filter preview

While a filter dialog is open, a parameter change SHALL render the filtered result on the canvas without recording a history state. Each update MUST be computed from the pre-filter pixels, not from the previous preview. A preview SHALL be bounded to the visible document section expanded by the filter's support, so previewing a large document costs at most a viewport; choosing OK MUST commit exactly one history state for the filter by filtering the whole layer; choosing Cancel MUST restore the pre-filter pixels bit-identically and record no history state.

#### Scenario: Preview does not record history

- **WHEN** a parameter is changed in the dialog
- **THEN** the canvas shows the filtered result and the history list is unchanged

#### Scenario: The live preview is bounded to the visible section

- **WHEN** a parameter changes on a large document
- **THEN** only the visible section plus the filter's support apron is filtered, and the commit still filters the whole layer

#### Scenario: OK commits one state

- **WHEN** the dialog is accepted
- **THEN** exactly one filter history state is recorded and the canvas keeps the result

#### Scenario: Cancel restores the pre-filter pixels

- **WHEN** the dialog is cancelled after previewing
- **THEN** the pre-filter pixels are restored bit-identically and no history state was added

### Requirement: Filter menu wiring

Every `Filter`-menu entry whose kind has a descriptor MUST be enabled and MUST dispatch its parameter dialog, or apply directly when it has no parameters. A dialog-opening entry's menu label MUST end with an ellipsis (`…`); a parameterless entry MUST not. Entries whose kind has no engine kernel MUST remain disabled. `Custom` is a normal parameterized kind and opens its dialog on the pass-through kernel. A filter command MUST be disabled when the active layer is not an unlocked normal pixel layer.

#### Scenario: An implemented entry is enabled and dispatches

- **WHEN** a document has an unlocked normal pixel layer active and the user opens the Filter menu
- **THEN** implemented entries are enabled and choosing one runs its filter or dialog

#### Scenario: Dialog entries carry an ellipsis

- **WHEN** the Filter menu is shown
- **THEN** every entry that opens a parameter dialog ends with `…`, and every parameterless entry does not

#### Scenario: An unimplemented entry stays disabled

- **WHEN** the user opens the Filter menu
- **THEN** entries with no engine kernel, such as `Filter > Video > De-Interlace`, are disabled

#### Scenario: No pixel layer disables the filters

- **WHEN** the active layer is missing or is not an unlocked normal pixel layer
- **THEN** the filter commands are disabled

### Requirement: Last Filter and Last Filter Settings

The `Filter` menu SHALL provide `Last Filter` (`Ctrl+F`) and `Last Filter Settings` (`Alt+Ctrl+F`). After a filter is committed, `Last Filter` MUST re-apply the same kind and parameter values with no dialog, and `Last Filter Settings` MUST reopen that filter's dialog prefilled with the last values. Both commands MUST remain disabled until a filter has been committed, and the `Last Filter` label MUST name the last filter.

#### Scenario: Last Filter re-applies without a dialog

- **WHEN** Gaussian Blur has been committed and the user invokes `Last Filter`
- **THEN** Gaussian Blur applies with the same radius and no dialog opens

#### Scenario: Last Filter Settings reopens prefilled

- **WHEN** the user invokes `Last Filter Settings` after a filter
- **THEN** that filter's dialog opens with the previous parameter values

#### Scenario: Both are disabled before any filter runs

- **WHEN** no filter has been committed in the session
- **THEN** `Last Filter` and `Last Filter Settings` are disabled

### Requirement: Lens Blur dialog and depth source

`Filter ▸ Blur ▸ Lens Blur…` SHALL be enabled whenever filters are and SHALL open CS6's Lens Blur dialog with these controls in order: Preview (`Faster` checked / `More Accurate`), Depth Map Source (None / Transparency / Layer Mask / Selection), Blur Focal Distance (`0..=255`), Invert, Shape (`Triangle (3)` through `Octagon (8)`), Radius (`0..=100`), Blade Curvature (`0..=100`), Rotation (`0..=360`), Specular Brightness (`0..=100`), Specular Threshold (`0..=255`), Noise Amount (`0..=100`), Distribution (Uniform / `Gaussian` checked), and Monochromatic, plus a depth-map id and a noise seed carried through without widgets. The `lens-blur` kind SHALL take 15 slots in that order; an empty slot list SHALL default to `Faster, None, 126, off, Triangle (3), 50, 33, 191, 44, 93, 47, Gaussian, off, 0, 1`, and a list of the wrong length SHALL be refused without changing the document. The dialog SHALL preview the whole layer (the depth map is positional), and the Preview radio SHALL change sampling density only: a commit SHALL always use the More Accurate path. Before every preview and commit the system SHALL materialize the source slot into a depth map — Transparency from the active layer's alpha, Layer Mask from its mask, Selection from the document selection, None as no map — cropped to the active layer's rectangle, and SHALL carry its id in the hidden slot. An unavailable source (a layer with no mask, no active selection) SHALL refuse the commit with a message and leave the pixels unchanged. `Filter ▸ Last Filter` SHALL re-apply Lens Blur with the same settings without reopening the dialog, and `Last Filter Settings` SHALL reopen the dialog prefilled.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Lens Blur dialog opens on a document
- **THEN** its 15 slots are `0, 0, 126, 0, 0, 50, 33, 191, 44, 93, 47, 1, 0, 0, 1`, the source list reads `None`, `Transparency`, `Layer Mask`, `Selection`, the shape list starts at `Triangle (3)`, and the preview covers the whole layer

#### Scenario: A transparency source blurs out of focus

- **WHEN** Lens Blur with Source `Transparency`, focal 126, and radius 50 is committed on a layer whose alpha varies
- **THEN** pixels whose alpha is far from the focal value blur more than pixels near it, and the layer's pixels change

#### Scenario: An unavailable source refuses the commit

- **WHEN** Lens Blur with Source `Layer Mask` is committed on a layer that has no mask, or Source `Selection` with no active selection
- **THEN** the commit fails with a message and the pixels are unchanged

#### Scenario: A Faster preview still commits More Accurate

- **WHEN** Lens Blur is committed with Preview `Faster`
- **THEN** the stored last-filter settings carry `More Accurate`, and the result equals the same commit made with `More Accurate`

#### Scenario: Last Filter reuses the settings

- **WHEN** `Filter ▸ Last Filter` is chosen after a Lens Blur commit, or `Last Filter Settings` is opened
- **THEN** Last Filter applies Lens Blur again with the same slots and one more history state, and Last Filter Settings prefills the dialog with them

### Requirement: Lighting Effects workspace

`Filter ▸ Render ▸ Lighting Effects…` and `Filter ▸ Last Filter Settings` (when the last filter was Lighting Effects) SHALL open a dedicated Lighting Effects workspace, not the generic slot dialog. Its options bar SHALL hold a Presets menu (CS6's 17 styles, from `2 o'clock Spotlight` to `Triple Spotlight`, then `Custom`; `Default` selected), buttons that add a Spot, Point, or Infinite light, Reset, a Preview checkbox (on), Cancel, and OK. Below the bar, the picture SHALL fill the workspace with the rig rendered on it and on-canvas controls over it. The selected Spot SHALL show its ellipse, hotspot ellipse, and four axis handles. The selected Point SHALL show its radius ring. The selected Infinite light SHALL show its disc and end handle. Every light SHALL show its centre handle inside an Intensity ring. Dragging SHALL move, turn, stretch, re-hotspot, resize, aim, or change the intensity of the selected light. Pressing another light's centre SHALL select it, and Alt-dragging SHALL duplicate the selected light. To the right, a Properties panel SHALL hold the selected light's type, Color, Intensity, and Hotspot (disabled unless a Spot), then the rig's Colorize, Exposure, Gloss, Metallic, Ambience, Texture, and Height (disabled while Texture is None), each number with a slider in step. Below it, a Lights panel SHALL list each light (`Spot Light 1`, `Point Light 1`, …) with an eye that hides it, and a trash button that deletes the selected light. The rig SHALL keep 1 to 16 lights, with the add buttons disabled at 16 and the trash disabled at one. Choosing a preset SHALL load its rig, and any edit SHALL switch Presets to Custom. The workspace SHALL produce the `lighting-effects` slots for the whole rig. OK SHALL commit one history state. Cancel SHALL leave the document and history untouched.

#### Scenario: The workspace follows CS6's layout and defaults

- **WHEN** the workspace opens on a document
- **THEN** its values are the Default style's 22 slots, Presets shows 18 entries with Default selected, the type list reads Spot / Point / Infinite, Hotspot is enabled and Height disabled, Lights lists `Spot Light 1` with the trash disabled, the options bar with OK and Cancel sits above the picture, and Properties sits above Lights to the right of it

#### Scenario: Presets, lights, and properties edit the rig

- **WHEN** `RGB Lights` is chosen, Gloss is changed, Reset is pressed, a Point and an Infinite light are added, lights are added until the cap, and lights are deleted down to one
- **THEN** the rig holds three coloured Spots, Presets reads Custom after the edit and Default after Reset, the new lights are named and selected with the slot count following, the add buttons disable at 16, and the last light cannot be deleted

#### Scenario: On-canvas controls edit the selected light

- **WHEN** a Spot is dragged by its body, its long-axis handle, its short-axis handle, its Intensity ring, and beyond its ellipse, another light's centre is pressed, the selected light is Alt-dragged, and an Infinite light's end handle is dragged
- **THEN** the centre moves, size and width grow, the intensity follows the ring and the Properties field, the angle turns, the pressed light becomes selected, a copy is added and selected, and the Infinite light's angle and elevation follow the handle

#### Scenario: Commit, cancel, and reopen

- **WHEN** a Point is added and OK pressed from the Filter menu, a separate session is cancelled after choosing a preset, and Last Filter Settings is opened after a two-light commit
- **THEN** one history state is added with last params equal to the workspace's slots, the cancelled session leaves pixels and history unchanged, and the reopened workspace reads the committed rig with Presets at Custom

### Requirement: Displace dialog and map file

`Filter ▸ Distort ▸ Displace…` SHALL be enabled whenever filters are, and SHALL open CS6's Displace dialog: `Horizontal Scale:` and `Vertical Scale:` (percent, `-999..=999`, default 10), a `Displacement Map:` radio pair (`Stretch To Fit` checked, `Tile`), and an `Undefined Areas:` radio pair (`Wrap Around`, `Repeat Edge Pixels` checked). The dialog SHALL have no preview pane and no Preview checkbox, and SHALL leave the canvas untouched while open. After OK the system SHALL ask for the displacement map in a file dialog filtered to Photoshop files; Cancel there, an unreadable file, or a Bitmap-mode file SHALL abandon the filter with no history state, and an unusable file SHALL say so. The map SHALL be the file's flattened composite with its embedded profile ignored: the first channel drives horizontal and the second vertical, and a one-channel file drives both. The `displace` kind SHALL take 5 slots in order: Horizontal Scale, Vertical Scale, Displacement Map (`0` Stretch To Fit, `1` Tile), Undefined Areas (`0` Wrap Around, `1` Repeat Edge Pixels), and the map id. The dialog SHALL carry the id slot through without a widget. Map id `0` SHALL be the neutral map, and an id the store no longer holds SHALL refuse the commit without touching pixels. The store SHALL keep at least the most recent map, so `Filter ▸ Last Filter` re-applies Displace with the same map without asking for a file, and `Last Filter Settings` SHALL reopen the dialog with the last settings and ask for a map again.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Displace dialog opens on a document
- **THEN** its values are `10, 10, 0, 1, 0`, the radios read `Stretch To Fit`, `Tile`, `Wrap Around`, `Repeat Edge Pixels` in that order, and no preview pane or Preview checkbox is present

#### Scenario: A loaded map displaces the picture and Last Filter reuses it

- **WHEN** a flattened RGB PSD whose red channel is 255 and green channel 128 is loaded as a map and Displace is committed with its id at 10%
- **THEN** each pixel shows the colour from about 12.7 px to its right with its row unchanged, and `Filter ▸ Last Filter`, labelled `Displace`, adds one more history state that displaces again

#### Scenario: Unusable maps are refused

- **WHEN** a missing file, a non-PSD file, or a Bitmap-mode PSD is loaded as a map, or Displace is committed with an id the store never issued
- **THEN** the load returns no id, and the commit fails with the pixels unchanged

### Requirement: Filter dialog colour swatches

A colour parameter in a filter dialog SHALL show as a swatch filled with the colour, without the hex code as text. Clicking the swatch SHALL open the application colour picker, and a colour picked there SHALL repaint the swatch and update the preview.

#### Scenario: Neon Glow shows its glow colour as a swatch

- **WHEN** the Neon Glow dialog opens
- **THEN** its Glow Color control is a swatch filled with the default blue and carries no text

### Requirement: Artistic filter dialog defaults

Each Artistic dialog SHALL open on these defaults, and its kind with an empty slot list SHALL map to the same values: Colored Pencil 4 / 8 / 25; Cutout 4 / 4 / 2; Dry Brush and Fresco 2 / 8 / 1; Film Grain 4 / 0 / 10; Neon Glow 5 / 15 with a blue (0, 0, 255) glow; Paint Daubs 8 / 7 / Simple; Palette Knife 25 / 3 / 0; Plastic Wrap 15 / 9 / 7; Poster Edges 2 / 1 / 2; Rough Pastels 6 / 4 / Canvas / 100 % / Relief 20 / Light Bottom; Smudge Stick 2 / 0 / 10; Sponge 2 / 12 / 5; Underpainting 6 / 16 / Canvas / 100 % / Relief 4 / Light Top; Watercolor 9 / 1 / 1.

Colored Pencil, Rough Pastels and Watercolor SHALL show no foreground or background swatches; black and white stand in for the document colours. Rough Pastels and Palette Knife SHALL have no Seed control. Light Direction SHALL be a menu of the eight CS6 directions, placed below Relief with Invert below it, and a dialog of up to eight controls SHALL keep them in one column.

#### Scenario: Dialogs open on their defaults

- **WHEN** the Plastic Wrap, Palette Knife, Colored Pencil, Watercolor, Rough Pastels, and Underpainting dialogs open
- **THEN** their values are 15 / 9 / 7; 25 / 3 / 0; 4 / 8 / 25 / seed; 9 / 1 / 1 / seed; 6 / 4 / Canvas / 100 / 20 / Bottom / off; and 6 / 16 / Canvas / 100 / 4 / Top / off / seed

#### Scenario: Underpainting stays in one column

- **WHEN** the Underpainting dialog opens
- **THEN** every slider sits in the same column

### Requirement: Filter dialogs preview on open

A filter dialog with Preview checked SHALL render its preview on the canvas as soon as it opens, without any control being changed.

#### Scenario: Opening previews

- **WHEN** a filter dialog opens and no control is touched
- **THEN** the canvas shows the filtered preview

### Requirement: Lens Flare dialog

`Filter ▸ Render ▸ Lens Flare…` and `Filter ▸ Last Filter Settings` (when the last filter was Lens Flare) SHALL open a dedicated Lens Flare dialog, not the generic slot dialog. The dialog SHALL show a preview of the whole picture, aspect kept and letterboxed, with the flare drawn at the current settings under a crosshair that a click or drag moves. Next to the preview SHALL be OK, Cancel, and a Preview checkbox, on by default. Below the preview SHALL be a Brightness percentage field (10–300 %, default 100) above a slider that stays in step with it, and a `Lens Type` group of four radio buttons (50-300mm Zoom, 35mm Prime, 105mm Prime, Movie Prime; Zoom checked). The dialog SHALL produce the `lens-flare` slots in order: brightness, centre x, centre y, lens index. With Preview on, the canvas SHALL show a non-committing whole-layer preview, deferred during a slider drag until release. OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Lens Flare canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Lens Flare dialog opens on a document
- **THEN** its values are `[100, 0.5, 0.5, 0]`, the Brightness field ranges 10–300 with a `%` suffix, the four lens radios read in CS6's order with Zoom checked, Preview is checked, the buttons sit to the right of the preview, and Brightness and the lens group sit below it

#### Scenario: The preview pad follows the crosshair

- **WHEN** the crosshair is moved to (0.2, 0.3) and then to (0.8, 0.1)
- **THEN** the values carry the new centre, the pad image keeps the picture's aspect, and each time the pad is brightest at the crosshair

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, the centre is set to (0.25, 0.75), and OK is pressed, and Last Filter Settings is then opened after a `[180, 0.6, 0.4, 2]` commit, and a separate dialog is cancelled after a preview
- **THEN** one history state is added with last params `[100, 0.25, 0.75, 0]` and the flare at that centre, the reopened dialog reads `[180, 0.6, 0.4, 2]`, and the cancelled canvas equals its prior pixels

#### Scenario: The section preview matches the commit

- **WHEN** Lens Flare is previewed with a visible viewport that excludes the flare centre
- **THEN** the preview is rendered against the whole layer and equals the full-layer commit inside the viewport

### Requirement: Shear dialog

`Filter ▸ Distort ▸ Shear…` and `Filter ▸ Last Filter Settings` (when the last filter was Shear) SHALL open CS6's Shear dialog, not the six-slider form. The dialog SHALL show a square curve box ruled by a dotted 4×4 guide, with a smooth line running from the top edge to the bottom edge. Clicking the line SHALL insert a control point at the click; dragging a point SHALL move it; dragging an interior point out of the box SHALL remove it; the two end points SHALL stay on the top and bottom edges and SHALL slide horizontally only, and interior points SHALL stay between their neighbours. The box SHALL show one handle per control point the user placed, never a sampled lattice, and SHALL refuse points past `SHEAR_MAX_POINTS` (8). The line SHALL be a cubic Hermite through the points with Catmull-Rom tangents, so a two-point curve is a straight line and a multi-point curve bends without kinks. Below the box SHALL be an `Undefined Areas:` group of two radio buttons, `Wrap Around` first and `Repeat Edge Pixels` second, with `Wrap Around` checked. OK and Cancel SHALL stand to the right of the box. A live preview of the picture SHALL sit below, with no Preview checkbox and no zoom controls, as CS6's Shear has none; the pane SHALL composite the filtered active layer over the unchanged layers, so the other layers are never displaced with it. The dialog SHALL produce the `shear` slots in order: a control-point count (`2..=SHEAR_MAX_POINTS`), then that many `(position, offset)` pairs top to bottom with unused pairs zero, then the fill index (`0` Wrap Around, `1` Repeat Edge Pixels). OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Shear canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Shear dialog opens on a document
- **THEN** its values are a count of 2, the two end points `(-1.0, 0.0)` and `(1.0, 0.0)`, zero padding, and fill `0`, the curve is a straight line down the middle of the box with exactly two handles, the two `Undefined Areas` radios read `Wrap Around` and `Repeat Edge Pixels` with `Wrap Around` checked, the preview sits below the curve box, and no Preview checkbox or zoom control is present

#### Scenario: The curve box edits the control points

- **WHEN** a point is dragged to the right at the bottom of the curve box, and then a point is added midway and dragged out of the box
- **THEN** the bottom point's stored offset becomes positive, the top point's stays zero, the count grows by one and returns, and the removed point's effect disappears from the points

#### Scenario: The curve keeps the points the user placed

- **WHEN** a point is clicked onto the line and not dragged
- **THEN** the stored count grows by exactly one, the new `(position, offset)` pair is the click, and the other slots stay zero; clicking past the cap leaves the count at `SHEAR_MAX_POINTS`

#### Scenario: The control points round-trip through the slots

- **WHEN** the dialog is opened with a previously committed 18-slot value list
- **THEN** the curve box reconstructs exactly the stored count and `(position, offset)` pairs, and the radios show the stored fill

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, the curve is bent, and OK is pressed, and Last Filter Settings is then reopened, and a separate dialog is cancelled after a preview
- **THEN** one history state is added carrying the 18 committed slots, the reopened dialog reads the same slots, and the cancelled canvas equals its prior pixels

#### Scenario: The preview is rendered against the whole layer

- **WHEN** Shear is previewed with a visible viewport smaller than the layer
- **THEN** the preview equals the full-layer commit inside the viewport

#### Scenario: The pane filters only the active layer

- **WHEN** the dialog's bottom pane previews Shear on a document with more than one visible layer
- **THEN** the pane shows the filtered active layer composited over the unchanged layers, so the other layers are not displaced with it

### Requirement: Reduce Noise kind and dialog

The filter mapping SHALL recognise `reduce-noise` with twelve slots: composite
Strength and Preserve Details, Reduce Color Noise, Sharpen Details, Remove JPEG
Artifacts, an Advanced flag, then the per-channel Strength/Preserve pairs for R,
G and B. An empty slot list SHALL default to Strength 6 with every other control
off and every channel equal to the composite; a non-empty list of any other
length SHALL be refused. `Filter ▸ Noise ▸ Reduce Noise` SHALL open a dedicated
dialog with Basic and Advanced pages: Basic edits one Strength and Preserve
Details pair for the whole image, Advanced reveals a Channel menu that selects
Red, Green or Blue and edits that channel's pair, and the Advanced flag SHALL be
set exactly when the Advanced page is active.

#### Scenario: The kind resolves and defaults

- **WHEN** `reduce-noise` is resolved with an empty slot list
- **THEN** the filter carries Strength 6, every other control off, and each
  channel equal to the composite

#### Scenario: Wrong arity is refused

- **WHEN** `reduce-noise` is resolved with a slot list of the wrong length
- **THEN** no filter is produced and the command returns false without changing
  the document

#### Scenario: The menu opens the dedicated dialog and commits

- **WHEN** `Filter ▸ Noise ▸ Reduce Noise` is chosen on an editable pixel layer
- **THEN** the Basic/Advanced dialog opens, and on OK one history state named
  `Filter: Reduce Noise` is recorded with the mapped slots

### Requirement: Picture Frame dialog

`Filter ▸ Render ▸ Picture Frame…` and `Filter ▸ Last Filter Settings` (when the last filter was Picture Frame) SHALL open a dedicated Picture Frame dialog, not the generic slot dialog. The dialog SHALL show a frame picker listing the 47 frames with a thumbnail each, a preview of the picture with the frame drawn at the current settings, a `Basic` page (vines colour, Margin, Size, Arrangement, Flower, flower colour and size, Leaf, leaf colour and size) and an `Advanced` page (Number of Lines, Thickness, Angle, Fade, Invert), and OK, Cancel, and a Preview checkbox (on by default). The dialog SHALL produce the `picture-frame` slots in order: frame, vine r/g/b, margin, size, arrangement, flower, flower r/g/b, flower size, leaf, leaf r/g/b, leaf size, lines, thickness, angle, fade, invert. With Preview on, the canvas SHALL show a non-committing whole-layer preview, deferred during a slider drag until release. OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Picture Frame canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Picture Frame dialog opens on a document
- **THEN** its values are the CS6 defaults (frame 34, vine `60 84 34`, margin 14, size 20, arrangement 1, flower 2, flower colour `0 88 218`, flower size 20, leaf 4, leaf colour `104 170 60`, leaf size 20, lines 15, thickness 29, angle 10, fade 0, invert off), the picker lists 47 frames with frame 34 selected, and there is a `Basic` page and an `Advanced` page with Preview checked

#### Scenario: The picker and controls drive the frame

- **WHEN** a different frame is picked, and then the flower is set to None
- **THEN** the frame slot carries the picked frame's 0-based index (index 0 is Happy Vine), the preview shows that frame, and setting the flower to None leaves the vine but draws no flowers

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, a frame and margin are set, and OK is pressed, and Last Filter Settings is then opened after a separate commit, and a separate dialog is cancelled after a preview
- **THEN** one history state is added with the matching last params and the frame drawn at those settings, the reopened dialog reads the committed params, and the cancelled canvas equals its prior pixels

#### Scenario: The section preview matches the commit

- **WHEN** Picture Frame is previewed with a visible viewport that excludes part of the border
- **THEN** the preview is rendered against the whole layer and equals the full-layer commit inside the viewport

### Requirement: Filter parameter descriptor

Every filter kind the project offers SHALL be recorded once, in the filter parameter descriptor. A descriptor SHALL record the kind's menu path, or for a kind with no menu row the family path it belongs to and that it is not a menu entry; an engine-only kind, which the engine applies but the Filter menu does not offer, SHALL instead record an empty menu path, `menu: false`, no gallery rank, and the shared filter-dialog policy, and is exempt from the family-path clause. The descriptor SHALL also record the kind's label, its ordered parameter controls, and for each control the control kind, its slot count, its label, its range, its choices, the value each of its slots opens on, and whether the control's value is a distance in document pixels. The descriptor SHALL also record whether the kind resolves against the whole layer, which filter dialog edits it, whether it asks for a displacement map after OK, and its dialog presentation flags (thumbnail pane, stacked layout, preview below, live preview).

The descriptor SHALL be the app's only table of a kind's dialog controls, canonical slot count, dialog-opening defaults, and pixel-distance facts. The mapping's variable-list decoders SHALL remain the only other place a variable kind's slot grammar is recorded. The shared parameter controls for a kind SHALL show exactly the controls its descriptor records, in the descriptor's order, each edited by the control the descriptor names, within the range the descriptor gives, with the choices the descriptor lists, opening on the values the descriptor records. A kind the Filter menu offers that owns a dedicated filter dialog SHALL open that dialog; the dedicated dialog's layout is its own requirement's, and the descriptor's controls record the slots it produces. An engine-only kind is not opened from this descriptor; its Image-menu dialog is governed by its own requirement. The mapping of a kind with an empty slot list SHALL build the filter from the descriptor's dialog-opening values, except that a commit uses the kind's commit normalization: Lens Blur's Preview radio is preview-only, so a commit always uses More Accurate. A kind with no descriptor SHALL not be a filter kind.

#### Scenario: The shared dialog opens on the descriptor's values [fpd_dialog_defaults]

- **WHEN** each kind edited by the shared parameter controls is opened
- **THEN** its controls, their order, their ranges, their choices and their opening values are the ones its descriptor records

#### Scenario: The mapping and the dialog agree on the defaults [fpd_mapping_defaults]

- **WHEN** each filter kind is mapped with an empty slot list and again with the descriptor's default vector
- **THEN** the two filters are equal, and a kind whose engine supplies a default produces that default

#### Scenario: The commit defaults are explicit [fpd_commit_defaults]

- **WHEN** Lens Blur is mapped with an empty slot list and then committed from its opening values
- **THEN** the mapped filter opens on Faster while the committed and stored slots carry More Accurate

#### Scenario: The menu rows are the descriptor's [fpd_cpp_rows]

- **WHEN** the filter menu is built
- **THEN** every menu row's kind, label, menu path, controls, slot counts, ranges, choices, default values, pixel-distance flags, dialog kind, map-after-OK flag and presentation flags are the ones the descriptor records for the same kind, and the descriptor with no menu row has no menu entry but still resolves through `filterCommandForKind`

#### Scenario: Engine-only rows are exempt from the family path [fpd_engine_only]

- **WHEN** the descriptor holds a kind the engine applies but the Filter menu does not offer
- **THEN** the row has an empty menu path and no gallery rank, and it is absent from the Filter menu and gallery

#### Scenario: Positional kinds come from the descriptor [fpd_whole_layer]

- **WHEN** a kind is asked whether it resolves against the whole layer
- **THEN** the answer is the one its descriptor records, and a kind with no descriptor is not a filter kind

#### Scenario: A kind opens the dialog the descriptor names [fpd_dialog_kind]

- **WHEN** a kind that owns its own dialog is chosen from the filter menu
- **THEN** that dialog opens for it, and a kind the descriptor names no dialog for that takes parameters opens the shared parameter dialog
