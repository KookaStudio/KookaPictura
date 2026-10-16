# Menus

- **Spec ID:** `UI-002`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — a new **Type** menu was added and most text/type functionality moved there; the CS6 default keyboard-shortcut table and Help add `Type > Warp Text`, `Type > Paste Lorem Ipsum`, `Type > Font Preview Size`, `Type > Language Options`, `Type > Load/Save Default Type Styles`; the **3D** menu was reworked for the new 3D engine (Extended only); Windows gained New/Open by context-clicking document tabs; menu items can be hidden/coloured and shown temporarily.
- **Depends on:** `UI-001` application-frame, `UI-003` workspace-and-docks, `UI-004` toolbox-and-options-bar, `11-cross-cutting/localization.md`

> **This is a reference spec.** Every item confirmed directly by the fetched CS6
> Help reference (via its `Choose X > Y > …` instructions and shortcut tables),
> or by a CS6 Help-derived secondary reference, is marked **✓**. Items that
> remain unresolved are marked **`(unverified)`**. Shortcuts are Windows; Mac
> substitutes `Cmd` for `Ctrl` and `Option` for `Alt` unless noted. This document
> is intentionally exhaustive; the `(unverified)` set is the residual
> screenshot-diff backlog against CS6 (a handful of items, see Open questions).

## CS6 behavior

- Photoshop has **eleven top-level menus** (Standard and Extended alike):
  **File, Edit, Image, Layer, Type, Select, Filter, 3D\*, View, Window, Help**
  (\*the **3D** menu appears only in Photoshop Extended). This matches the CS6
  Help reference and CS6 tutorial sources. CS5 Extended's separate **Analysis**
  menu has no CS6 residual: its functionality is now the Count/Ruler tools and
  the `Window > Measurement Log` panel (CS6 Help, "Counting objects in an image"
  and "Measurement (Photoshop Extended)").
- **Menus are customizable.** `Edit > Menus…` (or
  `Window > Workspace > Keyboard Shortcuts & Menus > Menus` tab) opens the
  Keyboard Shortcuts and Menus dialog, which can **show/hide** or **colour**
  items for both **Application Menus** and **Panel Menus**. Hidden items can be
  revealed temporarily with `Show All Menu Items` or `Ctrl`/`Cmd`-clicking the
  menu, and permanently with `Window > Workspace > Essentials`. Menu colours are
  toggled with `Show Menu Colors` in Interface preferences. The `New in CS6`
  workspace highlights new menu items in blue.
- **Menus grey out** when a command is inapplicable to the current document,
  tool, selection, layer kind or mode (e.g. Save unavailable on an untitled
  read-only document; many filters unavailable in Bitmap/Indexed/Duotone or in
  32-bit; 3D commands absent in Standard). The Help documents specific cases:
  `Edit > Clear` acts as Cut when pixels are selected; `View > Zoom In/Out`
  disable at the zoom limits; `Edit > Undo` toggles to `Redo`; palette commands
  that cannot apply are disabled by the same rule as Illustrator/InDesign.
  A precise per-item enablement matrix is not yet sourced — this is the bulk of
  the remaining work for this spec.
- **Context menus** are distinct from the menu bar and show commands relevant to
  the active tool, selection, or panel (e.g. right-click the canvas with the
  Eyedropper for sample-size options; right-click a layer for layer commands;
  right-click the tool icon in the options bar for `Reset Tool`/`Reset All
  Tools`; right-click the canvas background for canvas colour / Snap settings;
  right-click a document tab for New/Open on CS6 Windows).
- **Platform split.** On Windows the menu bar sits in the application frame; on
  Mac the application menu (`Photoshop > …`) carries About/Preferences/Services/
  Hide/Quit and the product menus follow. See `UI-001`.

### Menu tree

Legend: **✓** confirmed in the CS6 Help or a CS6-derived reference;
**`(unverified)`** not confirmed. Items with neither are groupings. Shortcuts are Windows.

#### File

- New… — `Ctrl+N` ✓
- Open… — `Ctrl+O` ✓
- Open As… — `Ctrl+Alt+Shift+O` ✓
- Browse in Bridge… — `Ctrl+Alt+O` ✓ (shortcut sheet)
- Open As Smart Object… ✓
- Open Recent ▸ ✓ (list length set by `File Handling > Recent File List Contains`)
- Close — `Ctrl+W` ✓
- Close All — `Ctrl+Alt+W` ✓
- Close and Go to Bridge… — `Shift+Ctrl+W` ✓ (shortcut table)
- Save — `Ctrl+S` ✓
- Save As… — `Shift+Ctrl+S` ✓
- Check In… ✓ (appears only when Adobe Drive / Version Cue is connected; CS5 residue)
- Save for Web & Devices… — `Ctrl+Alt+Shift+S` ✓
- Revert… — `F12` ✓
- Place… ✓
- Import ▸ ✓ (Variable Data Sets ✓; WIA Support ✓; installed scanner/camera devices)
- Export ▸ ✓ (Data Sets As Files ✓; Render Video… ✓; Zoomify… ✓)
- Automate ▸ ✓ (Batch… ✓; Conditional Mode Change… ✓; Contact Sheet II ✓;
  Create Droplet… ✓; Crop And Straighten Photos ✓; Merge To HDR Pro… ✓;
  PDF Presentation ✓; Photomerge… ✓; Picture Package ✓)
- Scripts ▸ ✓ (Export Layers to Files ✓; Image Processor ✓;
  Layer Comps to Files ✓; Load Files into Stack ✓; Script Events Manager ✓;
  Statistics ✓)
- File Info… — `Ctrl+Alt+Shift+I` ✓
- Print… — `Ctrl+P` ✓
- Print One Copy — `Ctrl+Alt+Shift+P` ✓
- Exit / Quit — `Ctrl+Q` (Win) / `Cmd+Q` (Mac) ✓

#### Edit

- Undo / Redo — `Ctrl+Z` ✓
- Step Forward — `Shift+Ctrl+Z` ✓
- Step Backward — `Ctrl+Alt+Z` ✓
- Fade… — `Shift+Ctrl+F` ✓
- Cut — `Ctrl+X` ✓
- Copy — `Ctrl+C` ✓
- Copy Merged — `Shift+Ctrl+C` ✓
- Paste — `Ctrl+V` ✓
- Paste Special ▸ ✓ (Paste Into — `Shift+Ctrl+V` ✓; Paste Outside ✓)
- Clear ✓
- Check Spelling… ✓
- Find And Replace Text… ✓
- Fill… — `Shift+F5` ✓
- Stroke… ✓
- Seam-Aware Scale (Photoshop: "Content-Aware Scale") — `Alt+Shift+Ctrl+C` (default) ✓
- Puppet Warp ✓
- Free Transform — `Ctrl+T` ✓
- Transform ▸ ✓ (Again — `Shift+Ctrl+T` ✓; Scale, Rotate, Skew, Distort,
  Perspective, Warp ✓; Rotate 180°, Rotate 90° CW, Rotate 90° CCW,
  Flip Horizontal, Flip Vertical ✓ — CS6 Help "Rotate and flip")
- Transform Path ▸ ✓ (Again, Warp, and the transform commands)
- Transform Points ▸ ✓ (Again)
- Define Brush Preset… ✓
- Define Pattern… ✓
- Define Custom Shape… ✓
- Purge ▸ ✓ (Undo, Histories ✓, Clipboard, Video Cache, All)
- Adobe PDF Presets… ✓
- Presets ▸ ✓ (Preset Manager…; Migrate Presets… ✓; Export/Import Presets… ✓)
- Color Settings… — `Shift+Ctrl+K` ✓
- Assign Profile… ✓
- Convert to Profile… ✓
- Keyboard Shortcuts… — `Alt+Shift+Ctrl+K` ✓
- Menus… ✓
- Preferences ▸ ✓ (General — `Ctrl+K` ✓; Interface; File Handling ✓;
  Performance ✓; Cursors ✓; Transparency & Gamut ✓; Units & Rulers ✓;
  Guides, Grid, & Slices ✓; Plug-ins ✓; Type ✓; 3D ✓) — Mac: Photoshop menu

#### Image

- Mode ▸ ✓ (Bitmap, Grayscale ✓, Duotone ✓, Indexed Color ✓, RGB Color,
  CMYK Color, Lab Color, Multichannel, Color Table ✓; 8/16/32 Bits/Channel ✓)
- Adjustments ▸ ✓ (Brightness/Contrast; Levels ✓; Curves; Exposure; Vibrance;
  Hue/Saturation; Color Balance; Black & White; Photo Filter; Channel Mixer;
  Color Lookup; Invert ✓; Posterize; Threshold; Gradient Map;
  Selective Color; Shadows/Highlights ✓; HDR Toning ✓; Desaturate ✓;
  Match Color ✓; Replace Color ✓; Equalize ✓; Auto Tone/Contrast/Color)
- Auto Tone — `Shift+Ctrl+L` ✓
- Auto Contrast — `Alt+Shift+Ctrl+L` ✓
- Auto Color — `Shift+Ctrl+B` ✓
- Image Size… — `Ctrl+Alt+I` ✓
- Canvas Size… — `Ctrl+Alt+C` ✓
- Image Rotation ▸ ✓ (180°, 90° CW, 90° CCW, Arbitrary…, Flip Canvas H/V)
- Crop ✓
- Trim… ✓
- Reveal All ✓ (`Image > Reveal All` restores area cropped off the canvas)
- Variables ▸ ✓ (Define… ✓; Data Sets… ✓)
- Apply Data Set… ✓
- Trap… ✓
- Apply Image… ✓
- Calculations… ✓

#### Layer

- New ▸ ✓ (Layer — `Shift+Ctrl+N` ✓; Layer from Background… ✓;
  Group…; Group from Layers…; Layer via Copy — `Ctrl+J` ✓; Layer via Cut —
  `Shift+Ctrl+J` ✓; Background From Layer ✓)
- Duplicate Layer… ✓ (also `Duplicate Group…`)
- Delete Layer ✓ / Delete Hidden Layers ✓ (CS6 exposes these directly and as
  `Layer > Delete > Layer` / `Layer > Delete > Hidden Layers`)
- Layer Style ▸ ✓ (Blending Options… ✓; Drop Shadow…; Inner Shadow…;
  Outer Glow…; Inner Glow…; Bevel & Emboss…; Satin…; Color Overlay…;
  Gradient Overlay…; Pattern Overlay…; Stroke…; Copy/Paste/Clear Layer Style ✓;
  Global Light… ✓; Create Layers ✓; Scale Effects ✓; Hide All Effects ✓;
  Show All Effects ✓)
- Smart Filter ▸ ✓ (Disable Filter Mask ✓; Delete Filter Mask ✓)
- New Fill Layer ▸ ✓ (Solid Color…, Gradient…, Pattern…)
- New Adjustment Layer ▸ ✓ (Brightness/Contrast ✓, Levels ✓, Curves ✓,
  Exposure ✓, Vibrance ✓, Hue/Saturation ✓, Color Balance ✓, Black & White ✓,
  Photo Filter ✓, Channel Mixer, Color Lookup, Invert ✓, Posterize ✓,
  Threshold ✓, Gradient Map ✓, Selective Color ✓, Shadows/Highlights,
  HDR Toning, Color Balance, Solid Color/Gradient/Pattern via Fill)
- Layer Content Options… ✓
- Layer Mask ▸ ✓ (Reveal All ✓; Hide All ✓; Reveal Selection ✓;
  Hide Selection ✓; From Transparency ✓; Delete; Apply; Enable/Disable;
  Link/Unlink)
- Vector Mask ▸ ✓ (Reveal All, Hide All, Current Path, Delete,
  Enable/Disable, Link/Unlink)
- Create Clipping Mask — `Ctrl+Alt+G` ✓; Release Clipping Mask ✓
- Smart Objects ▸ ✓ (Convert to Smart Object ✓; New Smart Object Via Copy ✓;
  Edit Contents ✓; Export Contents…; Replace Contents…;
  Rasterize ✓; Stack Mode ▸ ✓)
- Video Layers ▸ ✓ (New Blank Video Layer ✓; New Video Layer From File ✓;
  Replace Footage ✓; Interpret Footage ✓)
- Rasterize ▸ ✓ (Type, Shape, Fill Content, Layer ✓, Layer Style, Video ✓, 3D)
- New Layer-based Slice ✓
- Group Layers — `Ctrl+G` ✓; Ungroup Layers — `Shift+Ctrl+G` ✓; Hide Layers
- Arrange ▸ ✓ (Bring to Front — `Shift+Ctrl+]`; Bring Forward — `Ctrl+]`;
  Send Backward — `Ctrl+[`; Send to Back — `Shift+Ctrl+[`) ✓
- Align ▸ ✓ / Align Layers To Selection ▸ ✓ (Top, Vertical Center, Bottom,
  Left, Horizontal Center, Right)
- Distribute ▸ ✓ (Top, Vertical Center, Bottom, Left, Horizontal Center, Right)
- Lock All Layers In Group… ✓
- Merge Layers — `Ctrl+E` ✓
- Merge Visible — `Shift+Ctrl+E` ✓
- Flatten Image ✓
- Matting ▸ ✓ (Defringe… ✓; Remove Black Matte ✓; Remove White Matte ✓)

#### Type (new in CS6)

- Panels ▸ ✓ — Character / Paragraph / Character Styles /
  Paragraph Styles (CS6 Help: "access the Character and Paragraph panels
  from the `Type > Panels` menu")
- Anti-Alias ▸ ✓ (None, Sharp, Crisp, Strong, Smooth)
- Orientation ▸ ✓ (Horizontal, Vertical)
- Convert To Point Text / Convert To Paragraph Text ✓
- Warp Text… ✓ (CS6 moved from `Layer > Type > Warp Text` in CS5)
- Rasterize Type Layer ✓
- Create Work Path ✓ / Convert to Shape ✓ (CS6, moved from `Layer > Type` in CS5)
- Font Preview Size ▸ ✓ (None, Small, Medium, Large, Extra Large, Huge)
- Language Options ▸ ✓ (Middle Eastern features ✓; East Asian features)
- Update All Text Layers ✓
- Replace All Missing Fonts ✓ (CS6, moved from `Layer > Type` in CS5)
- Paste Lorem Ipsum ✓
- Load Default Type Styles ✓ / Save Default Type Styles ✓
- Extrude to 3D ✓ (Extended only; also reachable from the Type options bar)
- Check Spelling… / Find And Replace Text… — **not** on the CS6 Type menu;
  they live in `Edit` (`Edit > Check Spelling`, `Edit > Find And Replace Text`) ✓
- Panels ▸ OpenType options — *(unverified)* whether CS6 exposes an `OpenType`
  Type submenu or only the Character panel menu

#### Select

- All — `Ctrl+A` ✓
- Deselect — `Ctrl+D` ✓
- Reselect — `Shift+Ctrl+D` ✓
- Inverse — `Shift+Ctrl+I` ✓
- All Layers — `Ctrl+Alt+A` ✓
- Deselect Layers ✓
- Similar Layers ✓
- Color Range… ✓
- Refine Edge… — `Ctrl+Alt+R` ✓
- Modify ▸ ✓ (Border… ✓; Smooth… ✓; Expand… ✓; Contract… ✓;
  Feather… — `Shift+F6` ✓)
- Grow ✓
- Similar ✓
- Transform Selection ✓
- Save Selection… ✓
- Load Selection… ✓

#### Filter

- Last Filter — `Ctrl+F` ✓
- Convert for Smart Filters ✓
- Filter Gallery… ✓
- Adaptive Wide Angle… ✓ (CS6)
- Camera Raw Filter… ✓ (CS6)
- Lens Correction… — `Shift+Ctrl+R` ✓
- Liquify… — `Shift+Ctrl+X` ✓
- Oil Paint… ✓ (CS6)
- Vanishing Point… — `Alt+Ctrl+V` ✓
- Blur ▸ ✓ (Field Blur ✓, Iris Blur ✓, Tilt-Shift ✓, Gaussian Blur ✓,
  Motion Blur, Radial Blur, Box Blur, Surface Blur, Lens Blur ✓, Smart Blur,
  Average)
- Brush Strokes ▸ ✓ (Accented Edges, Angled Strokes, Crosshatch,
  Dark Strokes, Ink Outlines, Spatter, Sprayed Strokes, Sumi-e)
- Distort ▸ ✓ (Displace ✓, Glass, Ocean Ripple, Diffuse Glow, Pinch, Polar
  Coordinates, Ripple, Shear, Spherize, Twirl, Wave, ZigZag)
- Noise ▸ ✓ (Add Noise, Despeckle, Dust & Scratches ✓, Median ✓,
  Reduce Noise ✓)
- Pixelate ▸ ✓ (Color Halftone ✓, Crystallize, Facet, Fragment, Mezzotint,
  Mosaic, Pointillize)
- Render ▸ ✓ (Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects ✓)
- Sharpen ▸ ✓ (Smart Sharpen ✓, Unsharp Mask ✓, Sharpen, Sharpen Edges,
  Sharpen More)
- Sketch ▸ ✓ (Bas Relief, Chalk & Charcoal, Charcoal, Chrome,
  Conté Crayon, Graphic Pen, Halftone Pattern, Note Paper, Photocopy, Plaster,
  Reticulation, Stamp, Torn Edges, Water Paper)
- Stylize ▸ ✓ (Diffuse, Emboss, Extrude ✓, Find Edges, Glowing Edges, Solarize,
  Tiles, Trace Contour ✓, Wind)
- Texture ▸ ✓ (Craquelure, Grain, Mosaic Tiles, Patchwork,
  Stained Glass, Texturizer)
- Video ▸ ✓ (De-Interlace, NTSC Colors)
- Other ▸ ✓ (Custom ✓, High Pass, Maximum, Minimum, Offset)
- Digimarc ▸ ✓ (Embed Watermark ✓, Read Watermark ✓)
- Extract… / Pattern Maker… ✓ (optional plug-ins; `Filter > Extract ✓`,
  `Filter > Pattern Maker ✓`)
- Browse Filters Online… *(unverified)* — no CS6 Help citation found

#### 3D (Photoshop Extended only)

CS6 reworked the 3D engine; each command below is named in the CS6 Help
(chapters "3D" / "3D painting (Photoshop Extended)").

- New 3D Layer from File… ✓
- New Mesh From Grayscale ✓ (depth-map presets, `3D > New Mesh From Grayscale`)
- New 3D Extrusion From Selected Path, Layer, or Current Selection ✓
- New Shape From Layer ▸ ✓ (Spherical Panorama ✓; single-mesh presets such as
  donut, sphere, hat ✓; full preset list *(unverified)*)
- New 3D Postcard From Layer ✓
- New Tiled Painting ✓
- New Layer From 3D File… ✓
- New Mesh from Layer ▸ *(unverified)* — CS6 Help documents `New Mesh From
  Grayscale`, not a `New Mesh from Layer` entry
- Make Work Path from 3D Layer ✓
- 3D Paint Mode ▸ ✓
- 3D Paint Falloff ▸ ✓
- Select Paintable Areas ✓
- Create UV Overlays ▸ ✓
- Reparameterize ✓
- Repoussé ▸ ✓ (Edit In Repoussé ✓; Split Repoussé Meshes ✓;
  Create Constraints From Selection or Work Path ✓)
- Ground Plane Shadow Catcher ✓
- Snap Object To Ground Plane ✓
- Render Settings… ✓
- Render For Final Output (a render *setting*, not a menu item) ✓
- Export 3D Layer… ✓
- Auto-Hide Layers For Performance ✓
- Rasterize ✓
- Merge 3D Layers *(unverified)* — 3D layers merge via `Layer > Merge Layers` /
  `Merge Visible`; no separate command was found
- Get More Content… *(unverified)*
- Ground Plane / Environment / Lights submenus — CS5-era; CS6 moved these to
  the 3D panel, with `Ground Plane Shadow Catcher` and
  `Snap Object To Ground Plane` as the menu equivalents

#### View

- Proof Setup ▸ ✓ (Working CMYK, Working Gray, etc.; Custom ✓;
  Color Blindness > Protanopia-type / Deuteranopia-type ✓)
- Proof Colors — `Ctrl+Y` ✓
- Gamut Warning — `Shift+Ctrl+Y` ✓
- Pixel Aspect Ratio ▸ ✓ (Custom Pixel Aspect Ratio…, Delete/Reset Pixel
  Aspect Ratio…, etc.)
- Pixel Aspect Ratio Correction ✓ (video / non-square documents)
- 32-bit Preview Options… ✓
- Zoom In — `Ctrl++` ✓
- Zoom Out — `Ctrl+-` ✓
- Fit On Screen — `Ctrl+0` ✓
- 100% — `Ctrl+1` ✓ (CS6 renamed `Actual Pixels` → `100%`; some CS6-era
  references still print `Actual Pixels`)
- 200% ✓ (CS6 high-DPI view)
- Print Size ✓
- Screen Mode ▸ ✓ (Standard Screen Mode ✓; Full Screen Mode With Menu Bar ✓;
  Full Screen Mode ✓)
- Rulers — `Ctrl+R` ✓
- Snap — `Shift+Ctrl+;` ✓
- Snap To ▸ ✓ (Guides, Grid, Layer, Slices, Document Bounds, All, None)
- Lock Guides — `Alt+Ctrl+;` ✓
- Clear Guides ✓
- New Guide… ✓
- Lock Slices ✓
- Show ▸ ✓ (Selection Edges ✓, Target Path ✓, Layer Edges ✓, 3D Axis ✓,
  Guides ✓, Grid ✓, Smart Guides ✓, Count ✓, Slices ✓, Notes, Pixel Grid ✓,
  All ✓, None ✓, Show Extra Options ✓)
- Extras — `Ctrl+H` ✓
- `Align` / `Align To` — not Photoshop menu items (they belong to Illustrator);
  removed from the CS6 tree

#### Window

- Options ✓ (toggles the options bar — `Window > Options`)
- Arrange ▸ ✓ (Cascade ✓; Tile ✓; Float in Window ✓; Float All in Windows ✓;
  Consolidate All to Tabs ✓; Match Zoom ✓; Match Location ✓; Match All ✓;
  New Window For [file] ✓)
- Workspace ▸ ✓ (Essentials ✓; New in CS6 ✓; 3D; Motion; Painting;
  Photography; Typography; Advanced 3D ✓; New Workspace… ✓;
  Delete Workspace… ✓; Reset [Workspace] ✓;
  Keyboard Shortcuts & Menus… ✓)
- Panels (each toggles its dock) ✓ for: 3D ✓, Actions ✓, Adjustments, Animation,
  Brush ✓, Brush Presets ✓, Channels ✓, Character ✓, Character Styles ✓,
  Clone Source, Color, Histogram ✓, History, Info ✓, Layer Comps ✓, Layers ✓,
  Measurement Log ✓, Navigator, Notes, Paragraph, Paragraph Styles ✓, Paths,
  Properties, Styles ✓, Swatches, Timeline, Tool Presets ✓, Tools
  (the list of open documents is appended at the bottom of the Window menu ✓).
  CS6 has no separate `Gradients`, `Patterns`, or `Libraries` panels; gradients
  and patterns remain pop-up pickers.
- Extensions ▸ ✓ (Mini Bridge ✓; other Adobe/third-party extensions)
- 3D ✓ (alias for `Window > 3D`)
- Application Frame (Mac only) — toggles the Mac application frame
  *(unverified in the CS6 Help; confirmed general CS6 behavior)*

#### Help

- Photoshop Help — `F1` ✓
- Photoshop Support Center ✓
- Full Product Family Help ✓
- Adobe Product Improvement Program *(unverified)*
- About Plug-in ▸ ✓
- System Info *(unverified)*
- About Photoshop / About Photoshop Extended ✓

> **App extension (not CS6).** *Help ▸ Developer ▸ Design System…* opens the
> in-app design-system browser: a modeless storybook over the real widgets
> (foundations, shell, controls, patterns) with a live brightness preview. It is
> a Kooka Pictura developer affordance, deliberately outside the CS6 Help menu.

### Panel-context and canvas-context menus

- **Tool icon in options bar:** right-click → `Reset Tool` / `Reset All Tools` ✓.
- **Canvas (Eyedropper / sampling tools):** sample-size options ✓ (CS6 added
  "ignore adjustment layers" and "current layer and below" to the Sample menu).
- **Canvas (Move):** lists the layers with pixels under the pointer (when Auto
  Select is off) plus `Isolation Mode`.
- **Canvas (paint tools):** right-click opens the Brush Preset picker (size,
  hardness, and the preset grid). This is a deliberate Kooka Pictura affordance,
  not CS6 parity: the corpus documents the picker as an options-bar control and
  the on-canvas size/hardness gesture as the `Alt+Right-drag` HUD. The picker is
  a non-grabbing overlay, so the brush cursor ring keeps tracking the pointer
  and a single press on the document paints and closes it.
- **Canvas background:** right-click → canvas colour ✓ (also `Space+F`).
- **Document tab:** New / Open on CS6 Windows ✓.
- **Layers / Channels / Paths panels:** layer/channel/path commands
  (`02-ui-ux/panels/*`).
- **Panel menu (☰ button):** per-panel options; openable even when the panel is
  minimised ✓.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Menu bar (Win) / app menu (Mac) | Menus | `Alt`, `F10` | See tree above |
| `Edit > Menus…` | Dialog | `Alt+Shift+Ctrl+M` ✓ | Show/hide/colour app + panel menus |
| `Edit > Keyboard Shortcuts…` | Dialog | `Alt+Shift+Ctrl+K` ✓ | Remap menu + tool shortcuts |
| `Window > Workspace > Keyboard Shortcuts & Menus…` | Dialog | n/a ✓ | Same dialog, two tabs |
| Context menu | Popup | right-click | Tool/selection/panel-specific |
| Function keys | Shortcuts | `F1`…`F12`, `Shift+F5/F6/F7` | See below |

Function keys from the CS6 Help (Windows): `F1` Start Help / Undo-Redo; `F2`
Cut; `F3` Copy; `F4` Paste; `F5` Brush panel; `F6` Color panel; `F7` Layers
panel; `F8` Info panel; `F9` Actions panel (`Option+F9` on Mac); `Shift+F5`
Fill; `Shift+F6` Feather Selection; `Shift+F7` Inverse Selection; `F12` Revert.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Menu item visibility | bool | visible | show / hide | Per custom menu set |
| Menu item colour | swatch | None | colour or None | Show Menu Colors gates display |
| Menu set | enum | Photoshop Defaults | named sets | Saved via Save Set / Save Set As |
| Shortcut set | enum | Photoshop Defaults | named sets | `Edit > Keyboard Shortcuts` |
| Context menu | popup | n/a | varies by context | Not user-configurable |
| Number of top-level menus | int | 11 (10 in Standard; 3D absent) | — | Extended adds 3D |
| Function-key range | enum | n/a | `F1`…`F12` + modified | See table |

## Algorithms & pipeline

Design proposal; the menu system is declarative and generated by Qt.

- **Single menu source of truth.** Menus are declared once in a data table
  (id, path, shortcut, enablement predicate, checked state, platform mapping).
  The Windows menu bar, the Mac application menu, the panel context menus and
  the Keyboard Shortcuts & Menus dialog all read the same registry. This is what
  makes customisation and temporary "show all" possible.
- **Enablement.** Each command carries a predicate over
  `(document, activeTool, selection, layerKind, colourMode, bitDepth, edition)`.
  A disabled command is greyed, not hidden; a hidden command is absent unless
  `Show All Menu Items` / `Ctrl`-click is used.
- **Custom menu sets.** A set stores per-item `{visible, colour}` keyed by a
  stable command id. Applying a set rebuilds the menus. `Reset` restores the
  Photoshop Defaults set.
- **Dynamic labels.** A few items change label/state: `Undo`↔`Redo`,
  `Window > 3D` toggle, `View > Show > …` check marks, `Window > Arrange >
  [Workspace Name]` reset label, `View > Proof Setup` active profile check.
- **Command dispatch.** Menu activation emits a `Command` intent to the command
  layer (`ARCH-005`); undoable commands create history records, view/toggle
  commands do not.
- **Localisation.** Command ids are stable; labels come from a translation
  catalogue (`11-cross-cutting/localization.md`). The `(unverified)` items must
  still be mapped to ids so translations and shortcuts survive.

## Rust module mapping

Proposals.

- `pictura_ui::menu::Registry` — command table: `CommandId`, `MenuPath`,
  `Shortcut`, `EnablePredicate`, `CheckedProvider`, `PlatformMap`.
- `pictura_ui::menu::CustomSet` — `{ CommandId -> {visible, colour} }`,
  serialization, apply/reset.
- `pictura_ui::menu::context` — context-menu builders keyed by
  `(surface, activeTool, selectionKind, itemKind)`.
- `pictura_ui::shortcuts` — shortcut sets, conflict detection, `Use Legacy
  Channel Shortcuts`, `Use Shift Key For Tool Switch`.
- `pictura_ui::menu::enable` — the predicate functions over document state.

Crossing types: `CommandId` (string/interned id), `MenuPath`, `Shortcut`
(sequence of key+modifiers), `CustomSet`. No document data crosses; enable
predicates query the document through the bridge and return `bool`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PicturaMenuBar` | `QMenuBar` | Windows-style top-level menus; hidden in Full Screen modes |
| `CommandRegistry` | `QObject` | Command table; creates `QAction`s with data-carrying ids |
| `MenuBuilder` | `QObject` | Rebuilds `QMenu`s from registry + custom set; wires enablement |
| `ShortcutManager` | `QObject` | `QShortcut`/`QAction` shortcut sets, conflict checks, platform mapping |
| `ContextMenuFactory` | `QObject` | Builds per-surface `QMenu` on `contextMenuEvent` |
| `KeyboardMenusDialog` | `QDialog` | `Edit > Menus` + `Edit > Keyboard Shortcuts` (tabs) |

`QAction` is the natural unit: it carries text, shortcut, enabled/checked state,
and emits `triggered` carrying the command id. Use a command id in
`QAction::setData` rather than string-matching action text, so translations and
custom sets stay stable. `QMenu::aboutToShow` recomputes enablement
predicates so menus are correct on every open.

## Data-model impact

- **Command enablement reads, never writes, the document.** It is recomputed on
  `aboutToShow` from current document/tool/selection state.
- **Menu custom sets and shortcut sets are preference state** (not document
  data): stored per user, optionally captured into a workspace
  (`UI-003`). They never enter PSD/XMP.
- **Command history:** undoable commands record history via `ARCH-007`; toggles
  and view commands are explicitly non-undoable. The menu is not a history
  record.
- **Stable ids matter for the future**: every menu item needs a
  fixed id before localisation and shortcut remapping can be frozen.

## Edge cases

- **No document open.** Document-requiring commands disabled; File/Edit core and
  Help remain.
- **Colour mode / bit depth.** Bitmap, Indexed, Duotone and 32-bit disallow
  large parts of Image/Layer/Filter; enablement must reflect this rather than
  failing at execution. (Help documents many such restrictions.)
- **Standard vs Extended.** The 3D menu and all 3D/video/measurement commands
  are absent in Standard, not merely disabled.
- **Smart Objects.** Filter commands become smart-filter commands; `Fade` and
  destructive-only items disable on a smart object.
- **No selection.** Selection-dependent Edit/Layer/Select items disable.
- **Custom set hides a needed item.** `Show All Menu Items` / `Ctrl`-click must
  still reach it; a "reset to defaults" path must exist.
- **Shortcut collisions.** Remapping can collide; the dialog must warn and the
  dispatcher must resolve deterministically.
- **Mac application menu.** About/Preferences/Services/Hide/Quit have no direct
  Linux equivalent; map to a menu bar `Help > About` and `Edit > Preferences`,
  and document the divergence.
- **Transient state.** Menu labels that change (`Undo`/`Redo`, active proof
  profile) must update without rebuilding the whole menu.

## Parity acceptance criteria

- Given a fresh install, all eleven top-level menus (ten in Standard) and their
  documented submenus exist, in the documented order.
- Given a Bitmap-mode document, image/layer/filter commands CS6 disables are
  greyed in the same states; given a Standard licence, the 3D menu is absent.
- Given `Edit > Menus` and a hidden item, the item disappears from the menu,
  `Ctrl`-click / `Show All Menu Items` reveals it transiently, and
  `Window > Workspace > Essentials` reveals it permanently.
- Given an item coloured in the Keyboard Shortcuts & Menus dialog with
  `Show Menu Colors` on, it renders in that colour; turning the preference off
  suppresses the colour.
- Given a saved custom menu/shortcut set captured into a workspace, switching to
  that workspace applies the set and switching away restores the defaults.
- Given `Edit > Keyboard Shortcuts` and a remapped command, the new shortcut
  fires the command and the old one does not.
- Given no open document, every document-requiring menu item is disabled and the
  application does not crash.
- Given a right-click on the canvas, the tool/selection-appropriate context
  menu appears; right-clicking the options-bar tool icon offers
  `Reset Tool`/`Reset All Tools`.
- Given `Window > Arrange > New Window For [file]`, a second window for the same
  document opens and both stay in sync.
- Given the CS6 `New in CS6` workspace with menu colours enabled, new items
  render in the highlight colour.
- Given a translated build, every visible menu label comes from the catalogue
  and command ids are unchanged.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help reference (downloaded and text-extracted). This is
  the primary source for the **✓** items: it contains hundreds of
  `Choose X > Y > …` instructions across File, Edit, Image, Layer, Type, Select,
  Filter, 3D, View, Window and Help, plus the default shortcut tables (tool
  keys, `Keys for using panels`, function keys) and the panel/menu customisation
  chapter (`Edit > Menus`, Hide/show, colour, temporary reveal, `Show All Menu
  Items`), the CS6 Type-menu announcement, the 3D menu commands, and the
  screen-mode commands.
- `https://itwiki.wpunj.edu/images/e/ee/Photoshop_CS6_Extended_-_Dacier.pdf` —
  secondary student tutorial summarising the CS6 menu-bar functions per top-level
  menu (File/Edit/Image/Layer/Type/Select/Filter/3D/View/Window/Help). Secondary.
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces` —
  secondary CS6 tutorial; corroborates workspace entries under `Window >
  Workspace` and the `New in CS6` menu-highlight behaviour. Secondary.
- `https://training-nyc.com/legacy/photoshop_cs6_all_keyboard_shortcuts_sheet.pdf`
  ("Adobe Photoshop CS6 Keyboard Shortcuts", Training NYC) — secondary; confirms
  `Close`/`Close All`/`Print One Copy`/`Quit`, `Edit > Menus` = `Opt+Shift+Cmd+M`,
  `Last Filter`/`Vanishing Point`/`Lens Correction`, the File/Edit/Image/Layer/
  Select/Filter/View/Window/Help sample commands, and the `Photoshop` Mac menu
  (`Hide Photoshop`, `Hide Others`, `Quit Photoshop`).
- `https://hchsadobeacademy.weebly.com/uploads/7/1/6/2/7162106/p1_intro_photoshop_workspace.pdf`
  (Adobe, "Overview of Adobe Photoshop CS6 workspace", © 2012) — primary-ish Adobe
  PDF; confirms `Window > Workspace > Essentials (Default)`, the workspace
  switcher (`New Workspace`, `Reset Essentials`), the Properties/Timeline/Mini
  Bridge panels, and `File > Browse In Mini Bridge`.
- `https://www.bapugraphics.com/blog/adobe-photoshop-type-menu` — secondary CS6
  write-up of the new Type menu (Panels, Anti-Alias, Orientation, OpenType,
  Extrude to 3D, Create Work Path/Convert to Shape, Rasterize Type Layer, Warp
  Text, Font Preview Size, Language Options, Update All Text Layers, Replace All
  Missing Fonts, Paste Lorem Ipsum).
- `https://www.easycomputerbd.com/2026/01/adobe-photoshop-cs6-view-window-menu-guide.html`
  — secondary CS6 walk-through of the View and Window menus (Pixel Aspect Ratio,
  Pixel Aspect Ratio Correction, 32-bit Preview Options, Snap To members, the
  Window panel list, `Window > Options`, `Window > Tools`).
- Adobe CS6 Help pages via the Wayback Machine:
  `web.archive.org/web/20131128145732/http://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html`
  and
  `web.archive.org/web/20140620093338/http://helpx.adobe.com/photoshop/using/workspace-basics.html`
  — same text as the reference PDF.
- SearXNG meta-search queries used to locate secondary sources (CS6 menu-bar
  overview; workspace presets; Type/3D/Help menu item lists). No facts taken from
  snippets alone.

Not parsed: live `helpx.adobe.com` (HTTP 403 from this environment); the
Internet Archive CDX API was intermittently offline and O'Reilly blocks
non-browser fetches.

## Open questions

- **Per-item enablement matrix.** The Help documents many greying rules
  piecemeal; a complete `command × document-state` enablement table is not
  sourced. Resolve with a CS6 screenshot/interaction pass for every command.
- **Exact leaf membership and ordering of Filter submenus.** The Help's `Choose`
  paths prove every CS6 filter name and the canonical Brush Strokes / Sketch /
  Texture / Video groupings, but not the on-screen ordering. Resolve with a CS6
  `Filter` menu capture per submenu.
- **`3D` menu ordering and residual items** (`Merge 3D Layers`,
  `Get More Content…`, whether `New Mesh from Layer` survived the rename to
  `New Mesh From Grayscale`, and the full `New Shape From Layer` preset list).
  Resolve with an Extended CS6 capture.
- **`Filter > Browse Filters Online…`** — existence in CS6 is unverified.
  Resolve with a CS6 `Filter` menu capture.
- **`Type > OpenType`** — whether CS6 exposes OpenType options as a Type submenu
  or only in the Character panel menu. Resolve with a CS6 capture.
- **Help menu tail** (`Adobe Product Improvement Program`, `System Info`) —
  listed by tertiary sources, not the fetched CS6 Help. Resolve with a CS6
  `Help` menu capture.
- **Mac application-menu contents.** The CS6 keyboard-shortcut sheet shows the
  `Photoshop` menu carrying `Preferences…`, `Hide Photoshop` (`Cmd+H`),
  `Hide Others` (`Opt+Cmd+H`), and `Quit Photoshop` (`Cmd+Q`); `Services` and
  `About Photoshop` placement is still uncaptured.
- **Default shortcuts for many non-tool commands.** Only the documented tables
  are sourced; the rest need a CS6 keybinding capture. Feeds
  `02-ui-ux/keyboard-shortcuts.md`.
