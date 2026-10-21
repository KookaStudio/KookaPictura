# 3D Tools

- **Spec ID:** `TOOL-060`
- **Status:** `Draft`
- **Parity tier:** `Non-goal (Linux)` — CS6 source feature is `Extended-only`; Kooka Pictura does not target exact parity (`OVR-003`).
- **New in CS6:** `Changed` — CS6 folded object and camera manipulation into the **Move tool** options bar, replaced CS5 Repoussé with **3D Extrusion**, added on-canvas controls, drag-able shadows, and consolidated the scene/camera tools.
- **Depends on:** `OVR-002` cs6-editions-and-constraints, `OVR-003` feasibility-and-non-goals, `TOOL-061` 3d-panel, `ARCH-002` document-model, `ARCH-003` qt6-ui-design, `ARCH-006` gpu-rendering-pipeline, `01-architecture/file-formats.md`, `05-layers/linked-and-embedded-objects.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 3D is a closed engine; every algorithmic statement
> is either sourced to the CS6 Help text or marked **inferred**.

## CS6 behavior

The 3D tools exist only in **Photoshop CS6 Extended**; Standard has no 3D menu, panel, or tool modes (`OVR-002`). They become active when a 3D layer is selected.

**3D object tools** change the model's position or scale while the camera stays fixed:

- **Rotate** — drag up/down to rotate around the object's **x-axis**, side-to-side around its **y-axis**. `Alt`(`Option`)-drag rolls the model.
- **Roll** — drag side-to-side to rotate around the **z-axis**.
- **Pan** — drag side-to-side to move horizontally, up/down to move vertically. `Alt`-drag moves in the **x/z** plane.
- **Slide** — drag side-to-side to move horizontally, up/down to move closer or farther. `Alt`-drag moves in the **x/y** plane.
- **Scale** — drag up/down to scale. `Alt`-drag scales in the **z** direction.
- `Shift` constrains Rotate, Pan, Slide, or Scale to one direction. An options-bar **Return To Initial Position** icon restores the initial view; numeric position/rotation/scale fields sit at the right of the bar.

**3D camera tools** move the view while the object stays fixed:

- **Rotate** — orbit the camera in x or y; `Alt`-drag rolls it.
- **Roll** — roll the camera.
- **Pan** — pan in x or y; `Alt`-drag pans in x/z.
- **Walk** — z translation and y rotation; `Alt`-drag walks in the z/x direction (z translation + x rotation).
- **Zoom** — changes field of view; maximum FOV is **180**. Zoom alone offers **Perspective Camera** (parallels converge), **Orthographic Camera** (no convergence, accurate-scale view), and **DOF** (depth of field: `Distance` and `Blur`).
- Options: `Return To Initial Camera Position`, save/delete a custom view, x/y/z camera coordinates. Preset camera views from the `View` menu are all orthographic.

**Move tool consolidation (CS6).** With a 3D layer active the Move tool options bar offers `Rotate`, `Roll`, `Drag`, `Slide`, `Scale` modes, cycled with `Shift+V` (the CS6 Help uses **Drag** where the standalone tool is **Pan**). The document border colour signals the active target: **gold = camera**, **blue = Environment**, **green = Scene**, **no border = Mesh**.

**The 3D Axis** is an on-canvas gizmo showing x/y/z orientation; it appears when any 3D tool is selected and requires OpenGL (`View > Show > 3D Axis`). Drag an axis tip to move, a curved segment to rotate, the centre cube to resize, the coloured cubes to compress/elongate, and the inter-axis region to constrain motion to a plane.

**Creating and editing 3D content.** `3D > New 3D Extrusion From Selected Path/Layer/Current Selection` extrudes type, selections, closed paths, shapes, or image layers (the CS5 *Repoussé* feature renamed). `3D > New 3D Layer From File` imports a model; if a ground plane was defined first with `Filter > Vanishing Point`, the object snaps to it. `3D > Merge 3D Layers` merges any number of 3D layers to share shadows/reflections. `3D > Make Work Path from 3D Layer` traces the alpha channel into a work path. `3D > Export 3D Layer` writes Collada `DAE`, Wavefront `OBJ`, `U3D`, or Google Earth 4 `KMZ`.

**Import formats:** DAE (Collada), OBJ, 3DS, U3D, KMZ. **Export caveats (sourced):** U3D preserves only Diffuse, Environment, and Opacity maps; OBJ saves no camera, lights, or animation; only DAE saves render settings.

**Why this is a likely non-goal for Linux parity.** The engine is closed and Adobe has documented no algorithm. Three independent reasons make parity unattractive: (1) scope — a full 3D authoring, painting, lighting, and ray-tracing stack adjacent to a 2D raster editor; (2) platform — CS6 3D depended on **OpenGL 2.0** and a 512 MB VRAM floor, and Adobe itself discontinued 3D in Photoshop 22.5 (2021), citing the cost of porting from OpenGL to native APIs such as Metal; (3) value — the feature is `Extended-only`, and the project's own non-goal register already excludes it (`OVR-003`). A standard **glTF/PBR** module could reproduce *some* import/preview workflows one day, but exact CS6 parity would remain a non-goal.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel (3D Object group) | Tool | `K` | Press/hold to pick Rotate, Roll, Pan, Slide, Scale |
| Tools panel (3D Camera group) | Tool | `N` | Press/hold to pick Rotate, Roll, Pan, Walk, Zoom |
| Move tool options bar | Mode buttons | `Shift+V` | CS6: Rotate / Roll / Drag / Slide / Scale (Extended only) |
| 3D object options bar | Numeric + icons | — | Position menu, Save/Delete position, x/y/z coordinates, Return To Initial Position |
| 3D camera options bar | Numeric + icons | — | View menu, Save/Delete view, camera coordinates, Return To Initial Camera Position |
| Document window (3D layer) | On-canvas gizmo | — | 3D Axis; object bounding box; light/shadow guides |
| `View > Show > 3D Axis` | Menu toggle | — | Requires OpenGL |
| `3D > New 3D Extrusion From …` | Menu | — | Path / Layer / Current Selection |
| `3D > New 3D Layer From File` | Menu | — | DAE/OBJ/3DS/U3D/KMZ; honours a Vanishing Point ground plane |
| `3D > Make Work Path from 3D Layer` | Menu | — | Traces layer alpha to a work path |
| `3D > Merge 3D Layers` | Menu | — | Merge any number of 3D layers |
| `3D > Export 3D Layer` | Menu | — | DAE / OBJ / U3D / KMZ |
| `3D > Render Settings` | Menu + dialog | — | Presets, face/edge/vertex/volume/stereo |
| `3D > Ground Plane Shadow Catcher` | Menu | — | Ground-plane shadows |
| `3D > Snap Object To Ground Plane` | Menu | — | Align shadows with objects |
| `3D > 3D Paint Mode` | Menu | — | Target texture map for painting |
| `Window > 3D` | Panel | — | See `TOOL-061` |
| `Window > Workspace > Advanced 3D` | Workspace | — | Arranges 3D panels |
| Modifier | Key | `Alt`/`Option` + drag | Secondary axis / roll variants |
| Modifier | Key | `Shift` + drag | Constrain to one direction |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Object mode | enum | Rotate | Rotate / Roll / Pan (Drag) / Slide / Scale | `Shift+V` cycles in the Move tool |
| Camera mode | enum | Rotate | Rotate / Roll / Pan / Walk / Zoom | Tools-panel camera group |
| Object X/Y/Z position | float (doc units) | 0 | unbounded | Options bar; saveable "position" |
| Object rotation (x/y/z) | degrees | 0 | unbounded | Rotate/Roll gestures |
| Object scale | percent | 100% | > 0% | Uniform; `Alt` scales z |
| Camera X/Y/Z position | float | view-dependent | unbounded | Options bar; saveable "view" |
| Field of view (Zoom) | degrees | view-dependent | 0–180 | Max FOV 180 (sourced) |
| Camera projection (Zoom) | enum | Perspective | Perspective / Orthographic | Preset views are orthographic |
| Depth of field (Zoom) | float + float | off | `Distance`; `Blur` | Animatable to simulate focus pulls |
| Lighting Effects-style light type | enum | — | Point / Infinite / Spot / Image-based | 3D lights, see `TOOL-061` |
| VRAM floor | MB | 512 | ≥ 512 for 3D | CS6 Extended requirement |
| OpenGL | enum | required | OpenGL 2.0-capable | 3D unavailable without it |

## Algorithms & pipeline

### Scene model

A 3D layer is a **scene graph**: one or more **meshes** (polygon soup with vertices/faces), **materials** (up to nine texture-map types; see `TOOL-061`), and **lights** (infinite/spot/point/image-based) plus an implicit camera. Object and camera tools are rigid-body transforms with optional non-uniform scale; "Pan/Slide/Walk" are constrained translations along different basis planes. This is standard 3D math (4×4 transforms, Euler/axis-angle rotation) and can be implemented with any linear-algebra library.

### Rendering paths (sourced)

CS6 offers three quality modes:

- **Interactive (Painting)** — renders with **OpenGL on the GPU**, high quality but without detailed reflections/shadows. Best for editing.
- **Ray Traced Draft** — renders on the **CPU** with draft reflections/shadows.
- **Ray Traced Final** — full CPU ray trace with reflections, refractions, shadows, image-based lighting, and colour bleed. Tiles are drawn during rendering; a click or spacebar interrupts. Preferences expose `Render Tile Size`, `High Quality Threshold`, and `Shadow Quality`.

The renderer is Adobe's closed **Ray Tracer**; the Help text says the engine was "improved" in CS5/CS6 but publishes no sampling, BRDF, or shadow algorithm. Mark every ray-traced behaviour as **behavioral parity only, algorithm TBD**.

### Render modes

Render Settings expose **Face**, **Edge**, **Vertex**, **Volume**, and **Stereo** checkboxes with style options `Solid` / `Unlit Texture` / `Flat` / `Constant` / `Bounding Box` / `Normals` / `Depth Map` / `Paint Mask`, plus `Crease Threshold` (0–180°), `Line Width`, `Remove Backfaces`, `Remove Hidden Lines/Vertices`, `Reflections`, `Refractions`, `Shadows`, and `Render For Final Output`. Installed presets include Default, Bounding Box, Depth Map, Hidden Wireframe, Line Illustration, Normals, Paint Mask, Shaded Illustration, Shaded Vertices, Shaded Wireframe, Solid Wireframe, Transparent Bounding Box (Outline), Two-Sided, Vertices, and Wireframe. Volume mode is primarily for DICOM stacks (an `Extended-only` concern).

### 3D Axis

An on-canvas manipulator whose handles map to translation, rotation, and scale along the world axes; requires OpenGL to display. Implementation is an overlay gizmo picking against axis geometry, not document pixels.

### Why exact CS6 parity is not attempted

Adobe's ray tracer, material shader, OpenGL display path, and DAE/U3D/KMZ codecs are closed and undocumented. No open implementation can guarantee pixel-level equality for reflections, shadows, or refraction. The project's stated stance (`OVR-003`) is that 3D may return only as an optional **glTF/PBR** preview/import module, never as CS6 parity.

## Rust module mapping

Proposed optional `pictura-3d` crate, compiled out of a `Core` build:

- `pictura_3d::scene::Scene` — `{ meshes: Vec<Mesh>, materials: Vec<Material>, lights: Vec<Light>, camera: Camera }`.
- `pictura_3d::scene::Mesh` — vertices, indices, UVs, normals, world transform; `NodeId` reference for the owning 3D layer.
- `pictura_3d::camera::Camera` — `{ projection: Perspective | Orthographic, fov: f32, dof: Option<Dof>, transform }`.
- `pictura_3d::tools::{ObjectTool, CameraTool}` — enums mirroring Rotate/Roll/Pan/Slide/Scale and Rotate/Roll/Pan/Walk/Zoom; each resolves a drag delta into a transform update.
- `pictura_3d::axis::AxisGizmo` — hit-testing and handle math for the 3D Axis.
- `pictura_3d::io` — glTF-first import/export; DAE/OBJ/3DS/U3D/KMZ only if a later ADR requires them.
- `pictura_3d::render::{wgpu_backend, cpu_raytrace}` — optional preview and offline ray trace (proposal; scope undecided).
- `pictura_core::Edition` — gates registration of all 3D tools behind `Extended`.

Data crossing the boundary: `NodeId`, 4×4 `Mat4`, `Camera`, tool-mode enums. No Qt types in `pictura_3d`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ThreeDObjectOptionsBar` | `QWidget`/`QToolBar` | Mode buttons (Rotate/Roll/Pan/Slide/Scale), Return To Initial Position, numeric x/y/z |
| `ThreeDCameraOptionsBar` | `QWidget`/`QToolBar` | Mode buttons (Rotate/Roll/Pan/Walk/Zoom), Return To Initial Camera Position, FOV, projection, DOF |
| `SceneView3D` | `QQuickView` + `QtQuick3D` | Optional 3D preview/host if the module is ever built; `View3D` scene, `PerspectiveCamera`/`OrthographicCamera`, `PrincipledMaterial`, lights |
| `AxisGizmoOverlay` | `QQuick3D` node or `QGraphicsItem` | 3D Axis handles and picking |
| `ModeSelector` | `QToolButton` group | Holds/hides the object and camera tool family like CS6 |

Widgets for the options bars (dense, keyboard-centric) and QML/Qt Quick 3D for any viewport, consistent with `ARCH-003`. **Caveat:** Qt Quick 3D is GPLv3-or-commercial, which is a licensing decision the project must make before building this surface.

## Data-model impact

- **Layer kind:** a 3D layer is a distinct node kind storing a scene reference; see `ARCH-002` document-model and `05-layers/linked-and-embedded-objects.md`.
- **Non-goal representation:** in a `Core` build the 3D layer must still be representable and **preservable opaquely** so opening and re-saving a CS6 Extended PSD does not lose data (`OVR-002`, `OVR-003`). Exact PSD keys are not asserted here — see Open questions.
- **Undo granularity:** one history state per committed object/camera transform, mesh move, or render-setting change; a long ray trace should not create per-tile states.
- **Serialization:** camera views and object positions are saveable named states in CS6; if reproduced they are document data. Camera lights/render settings are layer data.
- **No pixels in undo:** transform records store matrix + named state, not rendered pixels (matches `ARCH-006`).

## Edge cases

- **No OpenGL / no GPU.** 3D surfaces must be hidden or disabled (CS6 behaviour: the 3D menu can disappear when the GPU/OpenGL requirement is unmet). Never crash; report unsupported.
- **VRAM below 512 MB.** CS6 Extended's 3D feature disables itself; mirror that gate rather than degrade silently.
- **Single build, two editions.** `Edition::Standard` must hide `Window > 3D`, the 3D menu, and the object/camera tool modes; `Extended` shows them.
- **Cross-edition files.** Opening an Extended PSD with 3D layers in a Standard build must preserve the 3D data and report it as unsupported, not rasterise silently.
- **Legacy codecs.** 3DS/U3D/KMZ have weak or abandoned library support on Linux; import may be partial.
- **Ray-trace interruption.** A large document render must be cancellable and must not block the UI thread.
- **1-px / empty layers.** Extrusion of a zero-area path or a fully transparent selection must be rejected cleanly.
- **Colour modes.** Extrusion/Repoussé was RGB-only (grayscale converted to RGB; CMYK/Lab unsupported). Preserve that gate if implemented.
- **Undo across render modes.** Changing Quality (Interactive → Ray Traced Final) is a layer setting, undoable as a setting change, not a pixel bake.

## Parity acceptance criteria

Because the tier is `Non-goal`, these assert **preservation and graceful absence**, not 3D rendering parity:

1. Given `Edition::Standard`, no 3D tool, menu, or panel is reachable.
2. Given `Edition::Extended` without a usable OpenGL/GPU context, 3D surfaces are hidden or disabled with a user-visible explanation; the app does not crash.
3. Given a CS6 Extended PSD containing a 3D layer, opening it in any build preserves the 3D data byte-for-byte on re-save and reports the feature as unsupported.
4. Given the object-tool modifier matrix (plain drag vs `Alt`-drag vs `Shift`-drag), any future implementation maps each gesture to the documented axis/roll variant.
5. Given a non-goal ADR is accepted, the corpus contains no requirement that CS6 ray-traced output be reproduced.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: 3D object tools (Rotate/Roll/Pan/Slide/Scale and their axes/modifiers); 3D camera tools (Rotate/Roll/Pan/Walk/Zoom, FOV max 180, Perspective/Orthographic, DOF); 3D tools gallery; Move tool consolidation in CS6 and `Shift+V`; document-border colour legend; 3D Axis behaviour; import/export formats and their caveats; `3D >` menu commands; `Window > 3D` and `Workspace > Advanced 3D`; the Interactive (OpenGL) / Ray Traced Draft / Ray Traced Final quality model and Ray Tracer preferences.
- `https://prodesigntools.com/products/adobe-cs6-system-requirements.html` — Photoshop CS6 / Extended CS6 requirements: 256 MB VRAM (512 MB for Extended), OpenGL 2.0-capable system; Extended 3D unavailable on Windows XP.
- `https://theartcult.net/how-to-enable-3d-menu-in-photoshop-cs6-25050.html` — 3D requires the Extended build plus `Use Graphics Processor`; 512 MB VRAM floor; `Window > Workspace > Advanced 3D`; `Window > 3D`.
- `https://www.cgchannel.com/2021/08/adobe-cans-photoshops-3d-tools` — Adobe discontinued Photoshop 3D in 22.5 (2021), attributing it to the cost of porting from OpenGL to native APIs (Metal) and to its dedicated Substance 3D line; no concrete plans to reintroduce.
- `https://www.daz3d.com/3d-bridge-for-photoshop/` — the DAZ **3D Bridge** is a third-party plug-in connecting DAZ Studio to Photoshop (CS5 or higher), exporting `.u3d` for CS4 Extended; establishes DAZ as an integration, not an Adobe engine component.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release: "Mercury Graphics Engine for 3D", "New 3D Controls", and "New 3D Effects" listed under Extended.

Consulted as search-result snippets only (not individually fetched; community/third-party):

- `https://html.duckduckgo.com/html/?q=Photoshop+CS4+Extended+3D+engine+DAZ+Luxology+technology` — search results: DAZ 3D Bridge, CS4 Extended 3D/ray-tracer coverage. No result supported a claim that Adobe licensed **Luxology** technology for the Photoshop 3D engine; Luxology's documented Photoshop links are its own plug-ins (`imageSynth`, Flix).
- SearXNG queries for "Photoshop CS4 Extended 3D engine licensed Luxology technology ray tracer" and "Photoshop CS6 Extended 3D requirements OpenGL VRAM ray tracer" — snippet-level corroboration only.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern help pages inaccessible; the archived CS6 Help PDF was used instead.
- Adobe's 3D FAQ (`helpx.adobe.com/photoshop/kb/3d-faq.html`) — referenced via the CG Channel article but not fetched directly.

## Open questions

- **Luxology/DAZ engine provenance.** Was Adobe's CS6 ray tracer or OpenGL display path built with DAZ or Luxology technology? No fetched source supports this; DAZ is a bridge vendor and Luxology's Photoshop products are third-party plug-ins. *Resolves with:* an Adobe engineering/press statement or SDK-era documentation.
- **Exact PSD serialization of 3D layers.** The published format spec's 3D keys are not asserted here. *Resolves with:* the Adobe PSD/PSB format spec and `01-architecture/file-formats.md`; capture the keys used by CS6 Extended.
- **Is 3D worth an optional glTF/PBR module**, and at what scope (import/preview only vs authoring)? *Resolves with:* an ADR after the Core tier stabilizes (`OVR-003`).
- **Which legacy 3D formats must open at all** (DAE/OBJ/3DS/U3D/KMZ)? *Resolves with:* a format decision in `01-architecture/file-formats.md`.
- **Qt Quick 3D licensing** (GPLv3-or-commercial) versus a wgpu-only viewport. *Resolves with:* a licensing decision in `01-architecture/build-and-packaging.md`.
- **Ray-tracer feasibility.** Reproducing CS6 ray-traced output would require an unspecified renderer; only behavioral, not pixel, parity is plausible. *Resolves with:* an explicit parity-tolerance decision if the module is ever built.
