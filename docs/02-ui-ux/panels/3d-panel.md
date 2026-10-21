# 3D Panel

- **Spec ID:** `TOOL-061`
- **Status:** `Draft`
- **Parity tier:** `Non-goal (Linux)` — CS6 source feature is `Extended-only`; Kooka Pictura does not target exact parity (`OVR-003`).
- **New in CS6:** `Changed` — CS6 **streamlined** the panel to four filters (Scene / Meshes / Materials / Lights) and moved most settings into the contextual **Properties panel**; earlier CS5 text labels survive in the CS6 Help PDF (see Open questions).
- **Depends on:** `TOOL-060` 3d-tools, `OVR-002` cs6-editions-and-constraints, `OVR-003` feasibility-and-non-goals, `ARCH-002` document-model, `ARCH-003` qt6-ui-design, `TOOL-013` layers-panel (texture groups), `TOOL-011` properties-panel.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. The CS6 3D engine is closed; algorithmic statements
> are either sourced to the CS6 Help text or marked **inferred**.

## CS6 behavior

The **3D panel** appears when a **3D layer** is selected and exists only in **Photoshop CS6 Extended** (`OVR-002`). In CS6 it is deliberately lean: the top section lists the scene's components, and the **Properties panel** shows the settings for whatever is selected.

**Panel modes.** Four buttons at the top filter the component list:

- **Scene** — shows all components, plus the `Scene` entry itself. Scene settings cover the render preset, quality, paint target, global ambient colour, and cross sections.
- **Meshes** — shows meshes only; each mesh is a row with **information** (number of materials, textures, vertices, faces) and display flags.
- **Materials** — shows materials only; each material expands to its texture-map list.
- **Lights** — shows lights only.

There is no separate object or camera filter button in CS6. The camera is reached through the **Scene** entry → `Current View`; object and camera *transform* values live in the **Properties panel**, which has a **Coordinates** mode toggled with `V` (and grouped settings otherwise). This document uses *scene / object / camera views* as the conceptual grouping; the panel's literal filters are the four buttons above.

**CS6 interaction loop (sourced).**

1. Pick Scene, Meshes, Materials, or Lights.
2. Select one or more elements in the list.
3. Adjust in the Properties panel, or drag in the document window (dragging an object or light also selects it in the panel).

The **Properties panel** then renders the contextual settings: Environment (global ambient, image-based lights, ground-plane shadows/reflections), Scene (render presets such as Bounding Box and Wireframe, cross-section/surface/point options), Camera (FOV, depth of field, stereo for anaglyph/lenticular/side-by-side), Mesh (catch/cast shadows, extrusion, edit source), Materials (texture/bump maps), and Light (infinite/spot/point, colour, intensity, shadows). A **Render** button at the bottom of the Properties panel commits the scene.

### Scene settings

- **Render Settings** — a preset plus an `Edit` command; presets include Default, Bounding Box, Depth Map, Hidden Wireframe, Line Illustration, Normals, Paint Mask, Shaded Illustration, Shaded Vertices, Shaded Wireframe, Solid Wireframe, Transparent Bounding Box (Outline), Two-Sided, Vertices, and Wireframe.
- **Quality** — `Interactive (Painting)` (OpenGL/GPU, no detailed reflections/shadows), `Ray Traced Draft` (CPU, draft), `Ray Traced Final` (CPU, full reflections/shadows, tiled and interruptible).
- **Paint On** — target texture map for direct painting (mirrors `3D > 3D Paint Mode`).
- **Global Ambient Color** — base ambient light on reflective surfaces; interacts with per-material ambient.
- **Cross Section** — a planar slice through the model: `Plane` (colour/opacity), `Intersection` highlight colour, `Flip Cross Section`, `Offset` (default 0 = midpoint), `Tilt` (up to 360° on the other two axes), and `Alignment` (x/y/z). Each side of a cross section can carry its own render settings (the `Two-Sided` preset shows solid on one half, wireframe on the other).

### Mesh settings

Each mesh row exposes **Catch Shadows** (receive), **Cast Shadows**, **Invisible** (hide the mesh but keep its shadows), and **Shadow Opacity** (softness). The mesh position tools in the panel's lower section move/rotate/scale a single mesh without moving the whole model.

### Materials, texture maps, and lights

See `## Parameters & ranges` for the map list. Materials can be applied on canvas with the **3D Material Drop** tool (sample with `Alt`-click, then drop on another surface) and picked with **3D Select Material**. Material **presets** (steel, fabric, wood) ship with the app and can be created, renamed, deleted, saved, reset, loaded, or replaced. Texture maps can be created, loaded, opened for editing (as a **Smart Object** in its own document), deleted, and have their **Target / U,V Scale / U,V Offset** edited. Textures appear under the 3D layer in the Layers panel.

**Lights** support `Point` (all directions), `Spot` (cone with hotspot/falloff), `Infinite` (directional), and `Image-based` (an illuminated image mapped around the scene). Each light exposes `Intensity`, `Color`, `Image`, `Create Shadows`, `Softness`, and for point/spot lights `Hotspot`, `Falloff`, and inner/outer `Attenuation` (linear falloff between the limits). Lights can be positioned with Rotate/Pan/Slide, "Point Light at Origin", and "Move to Current View", and whole light sets can be saved/added/replaced as presets.

**Overlays.** The panel's bottom **Toggle** menu shows/hides `3D Ground Plane`, `3D Light` guides (point = ball, spot = cone, infinite = line), and `3D Selection` (materials outlined with a coloured line, meshes with a bounding box). The toggle is enabled only when **OpenGL** is available, and overlay colours come from the 3D preferences.

### Extended-only and non-goal status

The panel does not exist in CS6 Standard, and Kooka Pictura's non-goal register excludes CS6 3D parity (`OVR-003`). A `Core` build must keep the panel **unreachable** while still representing 3D layers in the model (see Data-model impact).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > 3D` | Menu → panel | — | Opens the 3D panel (Extended only) |
| Layers panel 3D layer icon | Double-click | — | Opens the 3D panel |
| `Window > Workspace > Advanced 3D` | Workspace | — | Arranges 3D panels (3D + Properties) |
| Panel top | Filter buttons | — | Scene / Meshes / Materials / Lights |
| Panel component list | Tree/list | — | Meshes/materials/lights; expand a mesh to reveal materials |
| Panel bottom | Toggle menu | — | 3D Ground Plane / 3D Light / 3D Selection |
| Panel bottom | Add/Delete light | — | New light type menu; delete selected light |
| Panel menu (fly-out) | Menu | — | Light presets: Save / Add / Replace Lights |
| Properties panel | Contextual settings | `V` | Toggles properties ↔ coordinates |
| Properties panel | Render button | — | Commits the scene |
| `3D > Render Settings` | Dialog | — | Preset edit; face/edge/vertex/volume/stereo |
| `3D > 3D Paint Mode` | Menu | — | Paint target (mirrors Scene → Paint On) |
| `3D > Ground Plane Shadow Catcher` | Menu | — | Ground-plane shadows |
| `3D > Snap Object To Ground Plane` | Menu | — | Align shadows |
| Canvas | On-image controls | right-click | Scene/mesh/light context properties |

## Parameters & ranges

### Scene

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Render preset | enum | Default | 15+ installed presets (see CS6 behavior) | Per-layer |
| Quality | enum | Interactive (Painting) | Interactive (Painting) / Ray Traced Draft / Ray Traced Final | Draft/Final are CPU |
| Paint On | enum | Diffuse | texture-map type | Target for 3D painting |
| Global Ambient Color | colour | — | colour picker | Interacts with material Ambient |
| Cross Section | bool | off | on / off | Enables slice |
| Cross Section Plane | colour + opacity | — | colour; opacity | Displays intersecting plane |
| Cross Section Intersection | colour | — | colour picker | Highlights intersected areas |
| Cross Section Offset | float | 0 | ± to beyond model | 0 = midpoint |
| Cross Section Tilt 1/2 | degrees | 0 | up to 360° | Rotates plane on the other two axes |
| Cross Section Alignment | enum | — | x / y / z | Plane ⟂ axis |
| Flip Cross Section | bool | off | on / off | Shows opposite side |

### Mesh

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Catch Shadows | bool | — | on / off | Receive shadows from other meshes |
| Cast Shadows | bool | — | on / off | Cast onto other surfaces |
| Invisible | bool | off | on / off | Hide mesh, keep shadows |
| Shadow Opacity | float | — | softness | Helps composite 3D over 2D |

### Materials & texture maps

CS6 documents **up to nine texture types** the panel can load; several other material properties may be supplied as a value or a map. Map entries appear in the panel's lower section and as texture groups in the Layers panel.

| Map / property | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Diffuse | map or colour | — | any 2D content | Base colour; paintable; swatch used if map removed |
| Opacity | map or slider | 100% | 0–100% | White map = opaque, black = transparent |
| Bump | map + strength | none | grayscale map; `Bump` strength | Lighter = raised; strength active only if map exists |
| Normal | map | none | RGB map | Encodes x/y/z surface normals; Photoshop uses world-space maps |
| Environment | map | none | spherical panorama | Visible in reflective areas |
| Reflection | map or value | — | 0–100% | Reflects scene + environment map |
| Illumination | map or colour | — | colour | Self-lit, ignores scene lighting |
| Gloss | map or value | — | value/grayscale map | Black map = full gloss, white = none |
| Shine | value | — | low = dispersed, high = crisp | Dispersion of reflected light |
| Specular | colour/value | — | colour | Specular colour |
| Ambient | colour | — | colour picker | Interacts with Global Ambient |
| Refraction | float | 1.0 | refractive index | Active when Ray Traced + Refractions |
| Texture Target | enum | layer | layer / composite | Edit Properties dialog |
| U/V Scale | float | 1.0 | repeating < 1 | UV tiling |
| U/V Offset | float | 0 | — | UV repositioning |

### Lights

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Light type | enum | Infinite | Point / Spot / Infinite / Image-based | Add via panel icon |
| Intensity | float | — | — | Brightness |
| Color | colour | — | colour picker | — |
| Image | file | — | bitmap / 3D file | Image-based lights; HDR recommended |
| Create Shadows | bool | on | on / off | Disabling improves performance |
| Softness | float | — | blur radius | Shadow edge falloff |
| Hotspot | float | — | — | Spot only; bright centre width |
| Falloff | float | — | — | Spot only; outer width |
| Use Attenuation | bool + Inner/Outer | off | full < Inner, zero > Outer, linear between | Point/Spot |
| Position tool | enum | Rotate | Rotate / Pan / Slide / Point Light at Origin / Move to Current View | Spot/point constraints apply |

## Algorithms & pipeline

The panel is a **view + controller** over the 3D scene graph; it owns no algorithms of its own. Its only computational work is:

- **Selection mapping** between the panel's tree rows and scene-graph node IDs (meshes, materials, lights, camera/`Current View`).
- **Property dispatch** to the Properties panel per element type (Environment/Scene/Camera/Mesh/Material/Light).
- **Render-setting evaluation**, which is Adobe's closed Ray Tracer for Ray Traced modes and the OpenGL path for Interactive mode (see `TOOL-060`). Behavioural parity only, algorithm TBD.
- **Cross-section** is a clip-plane/geometry-intersection operation with per-side material/render overrides; the exact Adobe implementation is closed.

## Rust module mapping

- `pictura_3d::scene::SceneGraph` — authoritative scene data; the panel reads/writes it through a selected-node handle.
- `pictura_3d::scene::{Mesh, Material, TextureMap, Light, Camera}` — node types with stable `SceneNodeId(u64)`.
- `pictura_3d::render::RenderSettings` — preset enum + custom face/edge/vertex/volume/stereo flags; consumed by the renderer.
- `pictura_3d::render::CrossSection` — `{ enabled, alignment, offset, tilt1, tilt2, plane_color, plane_opacity, flip }`.
- `pictura_3d::material::TextureType` — enum of the map types; `TextureMap { kind, source: MapSource, uv: UvTransform }`.
- `pictura_3d::light::{Light, LightKind}` — point/spot/infinite/image-based with type-specific fields.
- `pictura_3d::presets` — render presets and material/light presets (serde-serialisable).
- `pictura_ui_bridge::ThreeDSelection` — selected `SceneNodeId` + mode, marshalled to the panel.

Data crossing the boundary: `SceneNodeId`, `ThreeDViewMode` enum, small settings structs, colour values. No Qt types in `pictura_3d`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ThreeDPanel` | `QDockWidget` | Host; filter buttons + component tree + bottom toggles |
| `ThreeDFilterButtons` | `QToolButton` group | Scene / Meshes / Materials / Lights |
| `SceneTreeModel` | `QAbstractItemModel` | Meshes, materials, lights, camera/`Current View`; visibility eye column |
| `SceneTreeView` | `QTreeView` | Component list; expand mesh → materials |
| `MaterialPreviewWidget` | `QWidget`/`QLabel` | Material preview + preset pop-up grid |
| `PropertiesStack` | `QStackedWidget` | One page per element type (Environment/Scene/Camera/Mesh/Material/Light); properties ↔ coordinates toggle (`V`) |
| `CrossSectionWidget` | `QWidget` | Alignment/offset/tilt/flip controls |
| `LightsPresetMenu` | `QMenu` | Save / Add / Replace Lights |
| `ToggleOverlaysMenu` | `QMenu` | Ground Plane / Light guides / Selection |

Dense, docked, keyboard-centric controls → **Widgets**, consistent with `ARCH-003`. If a 3D viewport is ever added it would need QML + Qt Quick 3D and its GPLv3-or-commercial licence decision (`TOOL-060`).

## Data-model impact

- **Node kind:** 3D is a layer kind referencing a scene graph (`ARCH-002`); the panel edits scene data, not pixels.
- **Preservation:** a `Core` build must round-trip 3D scenes opaquely on PSD save (`OVR-002`/`OVR-003`); exact PSD keys are not asserted here (Open questions).
- **Texture maps as Smart Objects:** opening a texture map for editing creates a separate document linked to the map; the link must survive save/open.
- **Undo granularity:** one history state per committed panel action (add/delete light, apply material preset, change render quality, edit cross section). Live scrubby-slider drags coalesce into one state.
- **Selection is UI state**, not document data, and is never serialised.
- **Presets** (render, material, light, lights groups) are preference/preset-library data, not document data.
- **Overlay toggles** (ground plane, light guides, selection outlines) are view preferences.

## Edge cases

- **No OpenGL.** Overlay toggles (Ground Plane / Light guides / Selection) are disabled; the panel must not offer them silently.
- **Multiple 3D layers.** Render settings are **per layer**; the panel must bind to the selected layer and not leak settings across layers.
- **Changing Quality to Ray Traced Final.** Long CPU render; the panel must stay responsive, report progress in the status bar/tiles, and allow interruption.
- **Deleted/replaced textures.** Removing an external texture is recoverable via Load Texture; an internally referenced texture is restored via Undo/Step Backward.
- **More than nine texture types.** Extra textures appear in the Layers panel and Paint Mode list; the panel's fixed nine-map layout must not drop them.
- **Empty lists.** A scene with no lights or one mesh must render cleanly; delete buttons disable appropriately.
- **Standard edition.** The panel is absent; opening an Extended PSD preserves the scene and reports it unsupported.
- **CMYK/Lab extrusion inputs.** Extrusion is RGB-only (converts grayscale; rejects CMYK/Lab); the panel should disable unsupported actions.
- **Cross-section extremes.** Offset beyond the model bounds and tilt near 360° must not produce degenerate geometry or crash.

## Parity acceptance criteria

Tier is `Non-goal`; criteria assert structure and preservation:

1. Given `Edition::Standard`, `Window > 3D` and the Advanced 3D workspace are not reachable.
2. Given a 3D layer is selected, the panel exposes the four filters (Scene / Meshes / Materials / Lights) and each filter shows exactly the matching component kind.
3. Given one or more components are selected, the Properties panel switches to the matching page and `V` toggles properties ↔ coordinates.
4. Given a CS6 Extended PSD with a populated 3D scene, opening and re-saving preserves the scene data and reports 3D as unsupported in `Core`.
5. Given a scene with two 3D layers, changing the render preset on one leaves the other unchanged.
6. Given no OpenGL context, the overlay-toggle menu is disabled/hidden.
7. Given a texture map opened for editing, the resulting link round-trips through save/open.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: panel layout and the Scene/Meshes/Materials/Lights filters; component-list vs Properties-panel split; Scene settings (render presets, Quality, Paint On, Global Ambient, Cross Section and its alignment/offset/tilt/flip); Mesh settings (catch/cast shadows, invisible, shadow opacity); Materials and the texture-map catalogue (Diffuse, Opacity, Bump, Normal, Environment, Reflection, Illumination, Gloss, Shine, Specular, Ambient, Refraction); material drop/select and presets; texture create/load/open/delete/edit-properties (Target, U/V scale/offset); Lights (point/spot/infinite/image-based, intensity/colour/shadows/softness, hotspot/falloff/attenuation, positioning, guides, light presets); Render Settings face/edge/vertex/stereo and the installed preset list; `Window > 3D`, 3D-layer-icon double-click, `Workspace > Advanced 3D`; overlay toggles requiring OpenGL.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release: CS6 Extended 3D controls/effects under the Extended list.
- `https://prodesigntools.com/products/adobe-cs6-system-requirements.html` — 512 MB VRAM and OpenGL 2.0 requirements for Extended 3D.
- `https://theartcult.net/how-to-enable-3d-menu-in-photoshop-cs6-25050.html` — `Window > 3D` and `Workspace > Advanced 3D`; 512 MB VRAM.

Consulted as search-result snippets only (not individually fetched; community/third-party):

- SearXNG query for "Photoshop CS6 Extended 3D requirements OpenGL VRAM ray tracer" — corroborating snippets only.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern help pages inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **"Scene / object / camera views" wording.** The brief groups the panel as scene/object/camera; CS6's literal filters are Scene/Meshes/Materials/Lights, with camera settings under Scene → `Current View`. *Resolves with:* a decision to keep the CS6 filter names (recommended) or to present a restructured panel.
- **Stale CS5 labels in the CS6 PDF.** The section is titled "3D panel settings (Photoshop CS5 Extended)" and parts of the text describe CS5's top/bottom panel rather than CS6's streamlined panel + Properties split. *Resolves with:* a CS6 UI capture or the archived CS6 Help web page.
- **Exact set of the "nine" texture types.** The Help lists more than nine named map/property rows while stating nine types; which ones are loadable maps versus scalar properties is ambiguous. *Resolves with:* a CS6 material panel capture or the ExtendScript 3D DOM reference.
- **Exact PSD serialization of the 3D scene.** Not asserted here. *Resolves with:* the Adobe PSD/PSB format spec (`01-architecture/file-formats.md`).
- **Render preset parameter values.** Preset names are sourced; their exact face/edge/vertex option values are not. *Resolves with:* a CS6 render-settings dialog capture.
- **Is the panel worth building at all** (vs an opaque "unsupported 3D layer" representation)? *Resolves with:* the 3D ADR (`OVR-003`).
