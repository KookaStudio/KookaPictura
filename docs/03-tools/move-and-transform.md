# Move & Transform

- **Spec ID:** `TOOL-001`
- **Status:** `Draft`
- **Parity tier:** `Core` (Move, Free Transform, Transform submenu, Warp, Puppet Warp, Content-Aware Scale; Extended-only 3D Move modes excluded)
- **New in CS6:** `Changed` — CS6 added an **Interpolation** pop-up to the Free Transform options bar (per-operation resampling choice); the Move tool option previously named *Show Bounding Box* surfaces as **Show Transform Controls**.
- **Depends on:** `ARCH-002` document-model, `ARCH-006` gpu-rendering-pipeline, `ARCH-007` undo-history, `TOOL-002`/`TOOL-003`/`TOOL-004` (selection sources), `08-selection/selection-model.md`, `08-selection/transform-selection.md`, `05-layers/smart-objects.md`.

> All crate, module, widget, and type names below are **design proposals**. No code exists in this repository.

## CS6 behavior

The **Move tool** (`V`) repositions the contents of the active layer(s), layer groups, selections, and guides. With **Auto Select** enabled it also picks the layer under the pointer: choosing `Layer` selects the top layer whose pixels are under the cursor, `Group` selects the top group (clicking an ungrouped layer selects that layer). Without Auto Select, right-clicking the canvas lists every layer that has pixels under the pointer and lets the user pick one from the context menu.

The Move tool options bar also exposes:
- **Show Transform Controls** — displays the transform bounding box and handles on the active layer/selection so the same scale/rotate gestures as Free Transform are available without entering the command (per the CS6 Help "Transform freely" topic).
- **Align** buttons — Top/Vertical Centers/Bottom/Left/Horizontal Centers/Right edges, aligning selected layers to each other, or to the selection border when one exists (`Layer > Align` / `Layer > Align Layers To Selection`).
- **Distribute** buttons — the same six edges, spacing three or more selected layers evenly (`Layer > Distribute`).
- In Photoshop Extended only, the Move tool additionally exposes 3D object/camera modes (Rotate, Roll, Drag, Slide, Scale; `Shift+V` cycles). Those are an `Extended-only` concern and out of scope here.

Movement: drag on canvas; arrow keys nudge 1 px; `Shift`+arrow nudges 10 px. `Alt`(`Option`)-drag with the Move tool duplicates the selection/layer as it moves. Holding `Ctrl`(`Command`) temporarily activates the Move tool from another tool, except when Hand, Slice, Path, Shape, or any Pen tool is active.

**Transform** is the command family around the selection, layer, layer mask, vector mask, path, vector shape, selection border, or alpha channel:
- `Edit > Free Transform` (`Ctrl+T` / `Cmd+T`) applies rotate, scale, skew, distort, and perspective in one continuous operation, switching between them with modifier keys.
- `Edit > Transform` submenu: `Scale`, `Rotate`, `Skew`, `Distort`, `Perspective`, `Warp`, `Rotate 180°`, `Rotate 90° CW`, `Rotate 90° CCW`, `Flip Horizontal`, `Flip Vertical`, `Again` (`Ctrl+Shift+T`).
- When the target is a whole path the menu becomes **Transform Path**; for multiple selected path segments it becomes **Transform Points**.
- `Select > Transform Selection` transforms the selection border itself (not its contents).

All transformations happen around a fixed **reference point**, centred by default. It can be moved by clicking one of the nine squares in the options-bar reference-point locator or by dragging the point in the canvas (it may be placed outside the item). The options bar exposes numeric `X`/`Y` position (with a relative-positioning toggle), `W`/`H` scale percentages with a link-lock for aspect ratio, a rotation angle, and `H`/`V` skew angles. Free Transform can also warp via the **Switch Between Free Transform And Warp Modes** button.

**Warp** (`Edit > Transform > Warp`) deforms via a control-point mesh. A **Warp Style** pop-up selects a preset shape or `Custom`; dragging control points, mesh segments, or mesh interior edits the custom warp; a **Bend** and `X`/`Y` distortion fields give numeric control (disabled for `None`/`Custom`). The warp mesh and control points are toggled with `View > Extras`.

**Puppet Warp** (`Edit > Puppet Warp`) lays a triangulated mesh over the layer and lets the user drop **pins** that either move geometry or anchor it. Options: `Mode` (mesh elasticity), `Density` (mesh-point spacing), `Expansion` (outer-edge expansion), `Show Mesh`, `Pin Depth` (which overlapping pin is on top), and `Rotate` (`Auto`, or `Alt`-drag to rotate about a pin). `H` temporarily hides pins; `Delete` removes selected pins; `Alt`-click a pin with the scissors cursor removes it. It can also be applied to layer and vector masks.

**Content-Aware Scale** (`Edit > Content-Aware Scale`) resizes without uniformly scaling important content. Options: reference-point locator / X-Y position, `W`/`H` scaling percentage with maintain-aspect lock and relative positioning, `Amount` (ratio of content-aware to normal scaling), `Protect` (an alpha channel to protect), and `Protect Skin Tones` (Extended-tagged behaviour documented only in the version-independent Help text).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel | Tool (`V`) | `V` | Move tool; `Ctrl`/`Cmd` temporarily activates from other tools |
| Options bar | Checkbox + dropdown | — | `Auto Select` → `Layer` / `Group` |
| Options bar | Checkbox | — | `Show Transform Controls` |
| Options bar | Buttons | — | 6 Align buttons; 6 Distribute buttons (Distribute needs ≥3 layers) |
| Canvas context menu | Context menu | Right-click | Lists layers with pixels under pointer (when Auto Select off) |
| `Edit > Free Transform` | Menu | `Ctrl+T` | Continuous scale/rotate/skew/distort/perspective |
| `Edit > Transform > Scale` | Menu | — | Submenu item |
| `Edit > Transform > Rotate` | Menu | — | Submenu item |
| `Edit > Transform > Skew` | Menu | — | Submenu item |
| `Edit > Transform > Distort` | Menu | — | Submenu item |
| `Edit > Transform > Perspective` | Menu | — | Submenu item |
| `Edit > Transform > Warp` | Menu | — | Enters warp mode |
| `Edit > Transform > Rotate 180° / 90° CW / 90° CCW` | Menu | — | Fixed rotations |
| `Edit > Transform > Flip Horizontal / Vertical` | Menu | — | Mirror |
| `Edit > Transform > Again` | Menu | `Ctrl+Shift+T` | Repeats last transform |
| Free-Transform options bar | Button | — | `Switch Between Free Transform And Warp Modes` |
| Transform options bar | Widgets | — | Reference-point locator, X/Y, W/H, angle, H/V skew, Interpolation pop-up, Commit/Cancel |
| `Select > Transform Selection` | Menu | — | Transforms the selection border |
| `Edit > Puppet Warp` | Menu | — | Pin/mesh deformation |
| Puppet Warp options bar | Widgets | — | Mode, Density, Expansion, Show Mesh, Pin Depth, Rotate |
| `Edit > Content-Aware Scale` | Menu | — | Content-aware resize |
| `Edit > Preferences > General` | Preference | — | Default **Image Interpolation** method |
| Modifier: duplicate while transforming | Key | `Alt`/`Option` + pick command | Duplicates the item |
| Modifier: free transform duplicate | Key | `Ctrl+Alt+T` | Starts Free Transform on a copy |
| Modifier: transform again duplicate | Key | `Ctrl+Shift+Alt+T` | Repeats transform on a copy |
| Refine Edge (from transform context) | Dialog | `Ctrl+Alt+R` | Shared with selection tools |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Auto Select | bool | off | on / off | Off by default; community-reported |
| Auto Select target | enum | Layer | Layer / Group | Only when Auto Select on |
| Show Transform Controls | bool | off | on / off | Replaces CS5 "Show Bounding Box" |
| Reference point | enum (3×3) | center | 9 anchor positions + draggable in-canvas | "Reference point locator" |
| X / Y position | float + unit | current | unbounded (doc coords) | Relative-positioning toggle |
| W / H scale | percent | 100% | > 0 % | Link-lock maintains aspect ratio |
| Rotation angle | degrees | 0 | unbounded | `Shift` constrains drag to 15° steps |
| H / V skew | degrees | 0 | unbounded | Free Transform `Ctrl+Shift`-drag |
| Interpolation | enum | Bicubic | Nearest Neighbor / Bilinear / Bicubic / Bicubic Smoother / Bicubic Sharper / Bicubic Automatic | **New in CS6** in the options bar; same set as Image Size |
| Warp Style | enum | None | None / Custom / preset shapes | Exact CS6 preset list not enumerated in the Help PDF — see Open questions |
| Warp Bend | float | 0 | preset-dependent | Disabled for None/Custom |
| Warp X / Y distortion | float | 0 | preset-dependent | Disabled for None/Custom |
| Puppet Warp Mode | enum | Normal | Rigid / Normal / Distort | CS6 Help names `Distort`; full list community-reported |
| Puppet Warp Density | enum | Normal | Fewer Points / Normal / More Points | Help describes more/fewer points |
| Puppet Warp Expansion | int (px) | 0 | ≥ 0 | Expands/contracts outer mesh edge |
| Puppet Warp Show Mesh | bool | on | on / off | `H` temporarily hides pins |
| Puppet Warp Rotate | enum | Auto | Auto / manual | `Alt`-drag rotates about a pin |
| Content-Aware Scale Amount | percent | 100% | 0–100% nominal | Ratio of content-aware to normal scaling |
| Content-Aware Scale Protect | enum | — | none / alpha channel | Channel chosen from document |
| Content-Aware Scale Protect Skin Tones | bool | off | on / off | Version-independent Help text |
| Image Interpolation preference | enum | Bicubic | Nearest Neighbor / Bilinear / Bicubic / Bicubic Smoother / Bicubic Sharper | Governs destructive transforms and Smart Object transforms |

## Algorithms & pipeline

### Coordinate model

An in-progress transform is a mapping from the item's source space to document space, parameterised by a reference point. Scale/rotate/skew translate to a 3×3 affine matrix; **Perspective** and **Distort** require the full projective (homography) form. Free Transform accumulates successive operations into one matrix and applies the *cumulative* transform on commit, because each committed raster resample loses sharpness:

```text
state := identity (or homography)
on gesture: state := gesture_matrix * state
on commit:  resample once with state, using selected interpolation
```

Photoshop explicitly recommends performing several manipulations before applying, because each committed transformation resamples the raster and makes the image slightly less sharp.

### Interpolation / resampling

The CS6 Help defines the resampling kernels (shared with `Image > Image Size`): **Nearest Neighbor** (pixel replication, hard edges), **Bilinear** (2×2 weighted average), **Bicubic** (4×4 neighborhood, smoother tonal gradation, default), **Bicubic Smoother** (enlargement), **Bicubic Sharper** (reduction, with sharpening). CS6 added **Bicubic Automatic**, which picks smoother when scaling up and sharper when scaling down. A CS6-specific detail confirmed by a third-party CS6 review: the Free Transform options bar carries this menu for ordinary layers, but **Smart Object** transforms ignore it and use the preference default.

### Reference point and numeric fields

Bounding-box handles are computed in source space and mapped through the live matrix. The reference point is the fixed point of the transform; moving it changes the translation component without changing scale/rotation. Numeric W/H are percentages relative to the initial bounding box; X/Y are absolute document coordinates (or deltas when relative positioning is on).

### Warp

Warp is a **bicubic Bezier patch** deformation: a control grid (Photoshop's default visual mesh is the familiar small grid) is displaced by the user, and the patch interpolates the interior. Preset Warp Styles are parameterised by `Bend` and `X`/`Y` distortion. This is standard Coons/Bezier-patch surface deformation; the exact Adobe patch degree and preset parameters are closed. Mark as *behavioral parity only, algorithm TBD* for preset exactness.

**Implementation (ported from photocraft).** Kooka Pictura's Warp is ported from [PhotoCraft](https://github.com/storytold/photocraft) (Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors, MIT OR Apache-2.0, taken under MIT; see `NOTICE.md`):

- *Geometry* (`pictura_core::warp`, from photocraft `geom::warp`): each preset style is one bicubic Bezier patch whose control grid follows the bend, fitted to Photoshop renders. It is the identity at bend 0. These are approximations, not verified parity. A custom warp is a one-patch 4×4 control grid. Dragging an anchor carries its handles; dragging a handle turns the opposite one; a press elsewhere pulls the surface under the pointer.
- *Resampling* (`pictura_render::transform_layer_warp`, from photocraft `algo::warp`): the source is cut into cells a few pixels across, and each cell's two triangles are mapped forward and rasterized. Planes are sampled premultiplied. A shrinking warp area-averages its source footprint. Where the warp folds, the later cell wins. The layer mask is flattened onto its default colour.
- *Session* (from photocraft's Warp mode): `Edit > Transform > Warp`, or the options-bar toggle, starts a mesh over the layer's box and keeps the box's current transform, Distort and Perspective included, as a frame after the mesh. The options bar shows `Warp:` (style), the orientation toggle, and `Bend`/`H`/`V`. Commit applies the warp and the box together, as one `Free Transform` history state.
- *Out of scope:* Split Warp and the Grid menu (Photoshop CC 2020, not CS6), and smart-object and Warp Text warps through this path.

### Puppet Warp

Puppet Warp is a **triangulated mesh** with interactive pins. Moving a pin solves for a deformation that keeps mesh triangles as rigid as possible. The known family of algorithms is *As-Rigid-As-Possible* shape manipulation (Igarashi et al.) with a per-vertex log-polar / rigid-weighted solve; Adobe's exact solver and `Mode` elasticity weights are closed. Proposed implementation: Delaunay/regular triangulation at `Density`, pins as position constraints, iterative ARAP solve, `Expansion` as a border offset, `Rigid`/`Normal`/`Distort` mapping to stiffness weights. Mark inferred.

### Content-Aware Scale

Photoshop scales the "unimportant" pixels while protecting high-energy content. The publicly understood mechanism is **seam carving** (Avidan & Shamir) with forward energy, plus an energy bias from the `Protect` alpha channel and skin-tone detection; `Amount` blends content-aware and uniform scaling. Mark as *behavioral parity only, algorithm TBD*; seam carving is a community-documented match, not an Adobe-published algorithm.

### Align / Distribute

Align computes each layer's non-transparent bounding box from its pixels (or the selection border when aligning to a selection) and translates layer content so the chosen edges coincide. Distribute requires ≥3 items and equalises gaps between consecutive edge positions. These are simple 1-D solvers over per-layer bounds.

## Rust module mapping

- `pictura_transform::TransformOp` — enum `{ Scale, Rotate, Skew, Distort, Perspective, Warp(WarpParams), }` plus helper constructors for the fixed `Rotate180/90CW/90CCW`, `FlipH/V`.
- `pictura_transform::ReferencePoint` — 3×3 anchor enum + free `Point2` override.
- `pictura_transform::Interpolation` — enum matching the six resample methods; resolved from the options bar or the preference.
- `pictura_transform::Matrix` — `Affine2` fast path and `Homography3` for perspective/distort; `to_inverse()` for undo.
- `pictura_transform::warp::BezierPatch` — control-grid deformation; `WarpStyle` presets.
- `pictura_transform::puppet::PuppetMesh` — `{ vertices, triangles, pins, mode, density, expansion }`; `solve()` returns per-vertex displacements.
- `pictura_transform::content_aware` — `seam_carve(energy: &EnergyMap, protect: Option<&Mask>, amount: f32)`.
- `pictura_imageops::resample` — `resample(src, matrix, interpolation) -> TileStore`; separable kernels for bilinear/bicubic, per-pixel nearest.
- `pictura_tools::move` — `MoveTool`, `AutoSelectMode { Off, Layer, Group }`, `hit_test_layer(doc, pos)`.
- `pictura_tools::align` — `align(layers, edge)`, `distribute(layers, edge)`.
- `pictura_selection::bounds` — non-transparent pixel bounds per `NodeId` for align/distribute.

Data crossing the boundary: `NodeId(u64)`, `Matrix`/`Homography` as `[f64; 6]`/`[f64; 9]`, `ReferencePoint`, `Interpolation`, and `TransformRecord { target: NodeId, inverse: Matrix, source_tiles: TileSnapshot }`. Pixel tiles stay in the core and are not marshalled as Qt image types per tile.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MoveToolOptionsBar` | `QWidget` | Auto Select check + `QComboBox` (Layer/Group); Show Transform Controls check |
| `AlignDistributeBar` | `QToolBar`/`QWidget` | Six align + six distribute `QToolButton`s, enabled by selection count |
| `TransformOptionsBar` | `QWidget` | Reference point, X/Y/W/H/angle/skew spin boxes, interpolation `QComboBox`, commit/cancel |
| `ReferencePointWidget` | `QWidget` | 3×3 locator; custom `paintEvent`/hit test |
| `InterpolationComboBox` | `QComboBox` | Six resample methods; disabled for Smart Objects |
| `TransformOverlay` | `QGraphicsItem` | Bounding box, 8 handles, rotation zone, live matrix, reference point |
| `WarpMeshOverlay` | `QGraphicsItem` | Warp control grid, mesh drag, bend/X/Y binding |
| `PuppetMeshOverlay` | `QGraphicsItem` | Mesh triangles, pins, pin depth, rotate cursor |
| `PuppetWarpOptionsBar` | `QWidget` | Mode/Density `QComboBox`, Expansion `QSpinBox`, Show Mesh check, Rotate |
| `ContentAwareScaleOptionsBar` | `QWidget` | Amount slider, Protect channel combo, Protect Skin Tones check |

Widgets over QML for consistency with `ARCH-003`: these are dense, keyboard-centric, docked-bar controls. The overlays live in the `QGraphicsView` vector layer described in `01-architecture/qt6-ui-design.md`, above the GPU-composited canvas background.

## Data-model impact

- **Selection/layer masks** are the transform source and destination; transforming a selection writes a new coverage mask (see `TOOL-002`).
- **Undo granularity:** one history state per committed transform. Because a transform resamples pixels destructively, the undo record must either keep the pre-transform source tiles or (for the common affine case) apply the inverse matrix and accept resample round-trip error. Photoshop's history stores the previous state; proposal: keep the source `TileSnapshot` for lossless undo and store the matrix only for Smart Objects.
- **PSD/XMP:** a raster transform is destructive and not serialised as a transform. A **Smart Object** non-destructively stores its transform; PSD carries smart-object transform data in the `Trnf` resource. Puppet Warp pins and warp meshes are not serialised — they are interaction state and are rasterised on commit.
- **Reference point / interpolation** are tool state, persisted in preferences/tool presets, never in the document.
- Puppet Warp pins are interaction-only; on commit they vanish into pixels. Open question whether the app should retain them for re-editing (Photoshop does not).

## Edge cases

- **Background layer** cannot be transformed until converted to a regular layer.
- **Locked layers:** `Lock Position`, `Lock All`, `Lock Image Pixels` gate move/transform; type and shape layers lock transparency/image by default.
- **Selection present:** transforming with an active selection moves the selection's contents; `Select > Transform Selection` transforms the border instead. Selection edges must be transformed with half-pixel coverage to preserve marching-ants fidelity.
- **Bit depth:** 8/16/32-bpc. 32-bit float resampling must not clip or clamp; Bicubic Sharper's implicit sharpening can overshoot float ranges.
- **CMYK/Lab:** interpolation is per channel in the document's space; hue interpolation in Lab is naive per-channel, matching Photoshop.
- **1-px / tiny documents:** handles overlap; numeric entry required; rotation of a 1-px row can produce an empty bounding box — guard against zero-area transforms.
- **PSB / huge docs:** full-image resample may exceed memory; must use tiled/streaming resample with a memory budget and a tiled undo snapshot.
- **GPU unavailable:** transform preview and warp mesh rendering fall back to CPU; overlays are cheap vector work and should not require the GPU.
- **Smart Objects:** non-destructive; the interpolation menu is disabled and the preference default applies; smart filters must be re-evaluated after transform.
- **Puppet Warp** is unavailable on Background layers and on some layer types; recommend Smart Object conversion for non-destructive use.
- **Content-Aware Scale** does not work on adjustment layers, layer masks, individual channels, Smart Objects, 3D layers, video layers, multiple simultaneous layers, or layer groups.
- **Undo/redo** must re-apply or unwind the matrix without cumulative resample loss; repeated commit/resample is an accepted Photoshop behaviour but our undo should be lossless.

## Parity acceptance criteria

- Given a layer with a `Show Transform Controls` checkbox on, dragging a corner handle with `Shift` scales proportionally around the reference point; releasing commits a bounding box whose W/H match the drag within 1 px.
- Given `Edit > Free Transform`, pressing `Esc` restores the item pixel-exact (bit-identical) to the pre-transform state.
- Given a reference point moved to the top-left locator, a 90° rotation maps the bounding box consistently with CS6 (same corner stays fixed) within 1 px.
- Given `Interpolation = Nearest Neighbor`, scaling a checkerboard by 400% yields hard-edged replicated pixels with no new intermediate colours.
- Given `Interpolation` is changed on the Free Transform options bar, the committed result differs from the Bicubic result by more than the noise floor for a scale of 200% (i.e. the choice is actually honoured).
- Given a Smart Object, the Free Transform options bar interpolation control is disabled and the commit uses the preference default.
- Given three layers and `Layer > Distribute > Horizontal Centers`, the horizontal-centre spacing between adjacent layers is equal within 1 px.
- Given a selection and `Layer > Align Layers To Selection > Right Edges`, every selected layer's rightmost non-transparent pixel equals the selection's right edge within 1 px.
- Given `Edit > Puppet Warp` with two pins, moving one pin leaves the pinned anchor area (within its influence radius) displaced less than 1 px.
- Given a Background layer, Move/Transform are blocked with the same "convert to layer" affordance as CS6.
- Given a 32-bit document, applying a transform does not clamp values to `[0,1]` beyond the source range.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (Feb 2013). Established: Move tool Auto Select Layer/Group and context menu; Show Transform Controls; align/distribute commands; `Edit > Transform` submenu; reference point locator; Free Transform modifier matrix; Warp and Puppet Warp workflows; Content-Aware Scale options and exclusions; selection-tool shortcut and modifier tables; Puppet Warp keymap; Free Transform duplicate/again shortcuts; interpolations methods (Nearest/Bilinear/Bicubic/Bicubic Smoother/Bicubic Sharper); "Interpolation menu added to options bar for Free Transform" (What's New in CS6); Extended-only Move tool 3D modes.
- `https://academyclass.com/blog/free-transform-tool` — third-party CS6 review confirming the CS6 Free Transform interpolation dropdown, its method set including Bicubic Automatic, and that Smart Object transforms ignore it in favour of the preference default.
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_9_2_4.html` — mirrored older Adobe Help text for Lasso tool options (used for cross-checking Width/Edge Contrast/Frequency wording).

Consulted as search-result snippets only (not individually fetched; community-reported):

- `https://www.photoshopessentials.com/basics/selections/magnetic-lasso-tool` and `https://www.photoshopessentials.com/basics/selections/magic-wand-tool` — default `Tolerance` 32 for the Magic Wand; magnetic-lasso option descriptions.
- Multiple CS6/lasso tutorials surfaced via SearXNG — Magnetic Lasso defaults Width 10 px, Edge Contrast 10%, Frequency 57.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern help pages were inaccessible; the archived CS6 Help PDF was used instead.
- Adobe's official list of Warp Style presets and the exact Puppet Warp Mode/Density enumerations — not present in the fetched CS6 Help text.

## Open questions

- **Warp Style preset enumeration.** The fetched CS6 Help PDF does not list the individual Warp styles or their exact `Bend`/`X`/`Y` parameter ranges. Resolve by consulting a CS6-owned screen reference or the archived Adobe Help page for "Warp an item" and recording the exact pop-up contents.
- **Puppet Warp option defaults and ranges.** `Mode` default, `Density` labels, and `Expansion` numeric range are not in the fetched text. Resolve against a CS6 UI capture or the archived Help page.
- **Exact Puppet Warp solver and `Mode` elasticity weights.** Adobe's implementation is closed; the ARAP proposal is inferred. Resolve with a behavioral study and a reference implementation.
- **Exact Bicubic kernel.** Photoshop's Bicubic is not identical to the textbook cubic convolution (Catmull-Rom) and may differ in ringing/overshoot. Resolve with a resample comparison against a CS6 reference image.
- **Content-Aware Scale algorithm.** Seam carving is the community explanation; masking, `Amount` blending, and skin-tone detection are not sourced. Resolve with a public-analysis study or a stated parity tolerance.
- **Anti-aliasing of transformed selection edges.** How Photoshop computes partial coverage for transformed marching ants is unspecified. Resolve by comparing transformed selections against CS6.
- **Move tool `Auto Select` default.** The fetched text describes the option but not its shipped default. Resolve with a CS6 preference capture.
- **Whether to retain Puppet Warp pins / warp meshes for re-editing.** Photoshop discards them on commit; retaining them would be a non-parity enhancement. Decide in `08-selection/transform-selection.md`.
