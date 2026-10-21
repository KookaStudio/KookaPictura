# Brushes Panel

- **Spec ID:** `PAN-012`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds **Erodible Tip**, **Airbrush Tip**, and **Brush Pose** controls to the Brush panel, adds **Brush Projection** to Shape Dynamics, and changes Color Dynamics to vary per stroke with an **Apply Per Tip** opt-out. The panel's left-side option-set list and padlocks are otherwise unchanged from CS5.
- **Depends on:** `BRU-001` brush-engine, `BRU-002` brush-dynamics, `BRU-003` bristle-brushes, `BRU-004` mixer-brush-engine, `BRU-005` airbrush-and-flow, `BRU-006` brush-presets, `ARCH-003` qt6-ui-design, `ARCH-006` gpu-rendering-pipeline, `03-tools/brush-and-pencil.md`.

> This document owns the **Brush panel / Brush Presets panel UI surface**. Brush
> rendering is `BRU-001`; the dynamics semantics and parameter tables are
> `BRU-002`; bristle, mixer, airbrush engines are `BRU-003`/`BRU-004`/`BRU-005`;
> preset storage and `.abr` are `BRU-006`. All crate, module, widget, and type
> names are **design proposals**. The repository contains a working
> implementation; this file stays the behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not. Facts
> not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

The **Brush panel** (`Window > Brush`, shortcut `F5`) holds the tip options that control how the brush applies paint to an image. It opens from that menu, or by selecting a painting, erasing, toning, or focus tool and clicking the panel button on the left side of the options bar. The **Brush Presets panel** (`Window > Brush Presets`) is the preset browser (`BRU-006`); the two are usually docked together.

### Anatomy

From the CS6 Help figure labels (A–H):

| Label | Element |
|---|---|
| A | **Locked** padlock (tip-shape attributes locked) |
| B | **Unlocked** padlock |
| C | Selected **brush tip** |
| D | **Brush settings** (left-side option-set list) |
| E | **Brush stroke preview** |
| F | Pop-up (panel) menu |
| G | **Brush tip shapes** (shown when Brush Tip Shape is selected) |
| H | **Brush options** (right-side parameters for the selected set) |

### Option sets and dynamic checkboxes

The left side of the panel is a list of option **sets**. Selecting a set shows its parameters on the right. A checkbox to the left of each option set turns its options on or off without opening the set. The sets are the dynamics groups owned by `BRU-002`:

| Option set | Enables |
|---|---|
| **Brush Tip Shape** | Size, Spacing, Hardness, Angle, Roundness, Flip X/Y, tip type (always available) |
| **Shape Dynamics** | Size/Angle/Roundness jitter, Minimum Diameter/Roundness, Tilt Scale, **Brush Projection** (CS6) |
| **Scattering** | Scatter/Count + jitter + Control, Both Axes |
| **Texture** | Pattern, Scale, Mode, Depth/Min Depth/Depth Jitter, Texture Each Tip |
| **Dual Brush** | Second tip, Mode, Diameter, Spacing, Scatter, Count |
| **Color Dynamics** | FG/BG jitter, Hue/Sat/Brightness/Purity jitter, **Apply Per Tip** (CS6 opt-out) |
| **Transfer** | Opacity/Flow jitter + Control |
| **Brush Pose** | Tilt X/Y, Rotation, Pressure + **Override** (CS6) |
| **Mixer Brush** | Wet, Load, Mix, Flow, Sample reset (for the Mixer brush, `BRU-004`) |

Below the option sets sit the **standalone toggles** (`BRU-001`): **Noise**, **Wet Edges**, **Airbrush/Build-up**, **Smoothing**, and **Protect Texture**.

### Tip-shape variants

The parameters shown for **Brush Tip Shape** depend on the tip type (all sourced from the CS6 Help):

- **Standard / sampled** — Size, Use Sample Size (sampled tips only), Flip X, Flip Y, Angle, Roundness, Hardness (not for sampled tips), Spacing.
- **Bristle** — Shape, Bristles, Length, Thickness, Stiffness, Spacing, Angle, plus a Brush preview. Bristle previews **require OpenGL**.
- **Erodible (CS6)** — Size, Softness (rate of wear), Shape, Sharpen Tip, Spacing, plus a Brush preview and the **Live Brush Tip Preview** at the upper-left of the canvas.
- **Airbrush (CS6)** — Granularity, Spatter, Hardness, Distortion, plus tip-shape options (`BRU-005`).

### Preview and padlocks

- The **brush stroke preview** at the bottom "shows how paint strokes look with the current brush options."
- Hovering a preset in the **Brush Presets panel** "dynamically preview[s] brush strokes in the Brush panel" (`BRU-006`).
- The **lock/unlock** padlock on the selected tip locks tip-shape attributes so they are retained when another preset is selected; each option set can also be locked so its settings survive preset switches (`BRU-002`/`BRU-006`).

### Brush settings (panel) menu

The pop-up menu provides at least: **New Brush Preset**, **Rename Brush**, **Delete Brush**, **Clear Brush Controls**, and **Copy Texture to Other Tools**. "Choose **Clear Brush Controls** from the Brush panel menu" to clear all options changed for a preset **except brush shape settings**, at once. Library-level Save/Load/Replace/Reset live in the Brush Presets panel menu (`BRU-006`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Brush` | Panel | `F5` | Opens/toggles the Brush panel |
| Options bar panel button | Button | — | Opens the Brush panel for a painting/erasing/toning/focus tool |
| `Window > Brush Presets` | Panel | `F5` *(shared dock)* | Preset browser (`BRU-006`) |
| Option-set list | Checkbox + row | click | Enable/disable a dynamics set; select to edit |
| Brush Tip Shape | Option set | — | Standard/bristle/erodible/airbrush parameters |
| Brush options area | Parameter stack | — | Right side, follows the selected set |
| Brush stroke preview | Preview | hover preset | Live stroke preview |
| Brush tip preview | Preview | click | Bristle/erodible/airbrush; click to view from different sides |
| Lock / Unlock | Toggle | click | Locks tip-shape attributes across preset switches |
| Panel menu | Menu | — | New/Rename/Delete Brush, Clear Brush Controls, Copy Texture to Other Tools |
| Brush Presets panel menu | Menu | — | Load/Replace/Reset/Save Brushes, Stroke Thumbnail (`BRU-006`) |
| Delete / rename | Gesture | `Alt`-click / double-click | Delete / rename the selected preset (`BRU-006`) |
| Canvas (`[` / `]`) | Gesture | `[` `]`, `Shift+[`/`]` | Decrease/increase size; `Shift` adjusts hardness for supported tips |
| Panel keys | Keys | `,` `.` / `Shift+,` `.` | Previous/next brush size; first/last brush |

## Parameters & ranges

The per-parameter types, defaults, and ranges are the tables in `BRU-002` (Shape Dynamics, Scattering, Texture, Dual Brush, Color Dynamics, Transfer, Brush Pose) and `BRU-001` (tip shape / toggles). Panel-specific layout/behaviour parameters:

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Option-set enabled | bool | per set | on / off | Left-side checkbox |
| Selected option set | enum | Brush Tip Shape | Tip Shape / Shape Dynamics / Scattering / Texture / Dual Brush / Color Dynamics / Transfer / Brush Pose / Mixer Brush | Right side follows |
| Tip type | enum | Standard *(inferred)* | Standard / Bristle / Erodible / Airbrush / Sampled | Determines the tip-shape parameter set |
| Tip-shape lock | bool | unlocked | locked / unlocked | Padlock on the selected tip |
| Set lock | bool | unlocked | locked / unlocked | Per option set |
| Apply Per Tip (Color Dynamics) | bool | off | on / off | CS6: on restores per-stamp color variation |
| Override (Brush Pose) | bool | off | on / off | Maintains a static pose |
| Brush stroke preview | preview | live | continuous | Repaints on any change |
| Tip preview | preview | live | continuous | Requires OpenGL for bristle |
| Live Brush Tip Preview | overlay | on for erodible | on / off | Upper-left of the canvas (`BRU-002`) |

## Algorithms & pipeline

The panel is a **widget layer over the active brush state**; all rendering math is `BRU-001`–`BRU-005`.

- **State binding** — the panel binds to the selected preset's settings block (`BRU-006`) plus any padlocked overrides; edits mutate the working brush, and `New Brush Preset` captures the block.
- **Preview generation** — the stroke preview is rendered by the Rust brush engine onto a fixed squiggle path, typically on the GPU (`ARCH-006`); the tip preview renders a single tip footprint, and 3-D style tip rotation (bristle) is a GPU/viewport affordance.
- **Enable/disable propagation** — a set's checkbox gates whether that group contributes at all; unchecking must not discard its stored values (they are retained for re-enable), matching preset capture.
- **Preset switching** — locked sets/tips are reapplied on top of the newly selected preset; unlocked groups take the preset's values (`BRU-002`/`BRU-006`).
- **Clear Brush Controls** — resets every non-tip-shape parameter to defaults in one action.
- **Live Brush Tip Preview (erodible)** — an overlay that visualizes tip wear as the user paints (`BRU-002`).

## Rust module mapping

- `pictura_brush::settings` — `BrushSettings` (the full block) and per-set structs (shared with `BRU-001`/`BRU-002`).
- `pictura_brush::panel` — `BrushPanelModel { settings, enabled_sets, locked_sets, tip_locked, selected_set }`, `is_enabled(set)`, `set_enabled(set, on)`, `clear_controls()`.
- `pictura_brush::preview` — `render_stroke_preview(settings, path, size) -> ImageHandle`, `render_tip_preview(settings, size) -> ImageHandle`.
- `pictura_presets::brush` — `BrushPreset`, `PresetLibrary`, `ToolPreset` (`BRU-006`); the panel commits to / reads from it.
- `pictura_core::prefs` — padlock state persistence, last selected set, preview size (`02-ui-ux/preferences.md`).
- `pictura_core::command` — applying a brush preset to the active tool (app-state, not document History).

Crossing types: `BrushSettings`, `BrushTip`, `LockMask`, `ImageHandle`, `BrushPreset`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `BrushesPanel` | `QDockWidget` | Host; option-set list + parameter stack + previews + padlocks |
| `BrushOptionSetList` | `QListWidget` with checkable items | Dynamic checkboxes and selection of option sets |
| `BrushParamStack` | `QStackedWidget` | One parameter page per option set |
| `BrushTipShapePage` | `QWidget` | Standard/bristle/erodible/airbrush tip controls (swaps by tip type) |
| `BrushDynamicsPage` | `QWidget` (per set) | Sliders/jitter/Control widgets from `BRU-002` |
| `BrushStrokePreview` | custom `QWidget` | Renders the live stroke preview from a Rust `QImage`/texture |
| `BrushTipPreview` | custom `QWidget` | Tip footprint; 3-D rotation for bristle when OpenGL is available |
| `LockToggle` | `QToolButton` | Per-tip and per-set padlocks |
| `BrushPanelMenu` | `QMenu` | New/Rename/Delete, Clear Brush Controls, Copy Texture to Other Tools |
| `BrushPresetsPanel` | `QDockWidget` | Preset grid/list + display modes (`BRU-006`) |
| `BrushesController` | `QObject` | Applies edits to the active brush and captures presets |

Widgets, not QML: a dense, docked, high-frequency editing surface, consistent with `ARCH-003`. The stroke preview is a custom-painted `QWidget` fed by the Rust renderer; bristle tip previews may embed a `QOpenGLWidget`/QRhi view when OpenGL is present (`ARCH-006`).

## Data-model impact

- **No PSD fields.** Brush settings and presets are app/preset state, not document data (`BRU-006`).
- **Active brush** is tool state; switching tools retains each tool's own brush (`03-tools/brush-and-pencil.md`).
- **Preset capture:** `New Brush Preset` snapshots the full `BrushSettings` block; padlocks survive preset switches.
- **Preferences:** padlock state, last option set, preview toggles.
- **Undo:** brush edits are not document History states; a stroke commits to History as one state via `ARCH-009`.
- **ABR:** the panel never writes `.abr` directly; persistence is `BRU-006`.
- **Forward compatibility:** unknown descriptor keys from a loaded preset must be preserved opaquely by the settings block.

## Edge cases

- **Preferences reset / Reset Brushes** — unsaved presets are lost; warn first (`BRU-006`).
- **No OpenGL** — bristle tip previews and any 3-D tip rotation are unavailable; the panel must not silently show them.
- **Sampled tips** — Hardness is not adjustable; the control must be disabled, not silently ignored.
- **Erodible tips** — the Live Brush Tip Preview overlays the canvas; placement must not clip at the document edge or interfere with the cursor.
- **Mixer brush** — selecting the Mixer brush swaps in its own option set (`BRU-004`); the standard Transfer set must not be double-applied.
- **Padlock interaction** — locking a set then switching presets must retain that set's values and reapply them; unlocking must not lose them.
- **Clear Brush Controls** — must not clear tip-shape settings (Help is explicit) and must be undoable locally or warned.
- **Disabled set** — unchecking a set does not erase its values; re-checking restores them.
- **Huge/unknown preset** — a preset referencing a missing pattern (`Texture`) loads with Texture as a no-op (`BRU-006`).
- **Keyboard-only** — option-set checkboxes and all sliders must be reachable and adjustable without a pointer.
- **Panel resize** — narrow dock widths must not clip the stroke preview or the numeric fields.
- **Live preview cost** — high-frequency slider drags must coalesce preview re-renders to the UI frame rate (`ARCH-006`).

## Parity acceptance criteria

1. Given the Brush panel, the left side lists the option sets with checkboxes; clicking a checkbox enables/disables that set without opening it.
2. Given **Shape Dynamics** selected, the right side shows its parameters; setting `Size Jitter` to 100% changes the stroke preview accordingly.
3. Given a standard tip, the tip-shape page shows Size/Spacing/Hardness/Angle/Roundness/Flip; given a bristle tip, it shows Shape/Bristles/Length/Thickness/Stiffness/Spacing/Angle.
4. Given a sampled tip, the **Hardness** control is disabled.
5. Given the tip padlock locked, selecting a different preset retains the locked tip-shape attributes; unlocking stops that retention.
6. Given **Clear Brush Controls**, all non-tip-shape settings reset while Size/Spacing/Hardness and other tip-shape settings remain.
7. Given the pointer hovering a preset in the Brush Presets panel, the stroke preview updates to that preset without selecting it.
8. Given a bristle tip with OpenGL available, the tip preview renders and can be viewed from different sides; without OpenGL it is unavailable.
9. Given an erodible tip, painting shows the Live Brush Tip Preview at the upper-left and **Sharpen Tip** restores crispness.
10. Given Color Dynamics with **Apply Per Tip** on, color varies per tip stamp; with it off, color is constant within a stroke and varies between strokes.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: the Brush panel purpose and `Window > Brush` / options-bar panel button; the anatomy labels A–H (locked/unlocked, selected tip, brush settings, stroke preview, pop-up menu, tip shapes, brush options); the dynamic option sets and their left-side checkboxes; Brush Tip Shape options for standard, bristle, erodible (CS6), and airbrush (CS6) tips; the tip-shape preview and OpenGL requirement for bristle previews; the brush stroke preview; `Clear Brush Controls` (all options except brush shape); `New Brush Preset`; the padlock retention behavior; the standalone options Noise, Wet Edges, Airbrush/Build-up, Smoothing, Protect Texture; Mixer-brush option set; `F5` for Show/Hide Brush panel; "Keys for the Brush panel" (Alt-click delete, double-click rename, comma/period sizing, `Shift+Alt+P` airbrush toggle); the CS6 "What's New" entries for erodible tips, airbrush tips, brush pose, brush projection, and Color Dynamics Apply Per Tip.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Brushes_palette.html` — secondary (Adobe Photoshop CS6 for Photographers companion). Established: the option-set list revealing right-hand parameters, padlocking a set (e.g. Shape Dynamics) across preset switches, the stroke preview reflecting dynamics, and tool-preset creation from the Tool Presets panel / options bar.

Not used in this pass:

- `helpx.adobe.com` brush-panel pages and `simonsezit.com` CS6 brush article (the latter returned only site navigation). The archived CS6 PDF and `BRU-001`–`BRU-006` were used instead.

## Open questions

- **Default option set on open** (Brush Tip Shape assumed) and the default enabled sets. *Resolves with:* a CS6 first-run capture.
- **Exact panel-menu item list.** The Help names New/Rename/Delete, Clear Brush Controls, and Copy Texture to Other Tools; the full CS6 menu may include more. *Resolves with:* a CS6 Brush panel-menu capture.
- **Per-set padlock UI location** — whether padlocks sit beside each option set or only on the tip. *Resolves with:* a CS6 capture.
- **Airbrush tip parameter ranges** (Granularity/Spatter/Hardness/Distortion) — named but not ranged in the Help. *Resolves with:* `BRU-005` and a CS6 capture.
- **Erodible tip `Shape` option list.** *Resolves with:* a CS6 capture and `BRU-002`.
- **Whether the Brush panel and Brush Presets panel share one dock/shortcut.** *Resolves with:* a CS6 workspace capture.
- **Bristle tip preview rendering path** (OpenGL vs software fallback). *Resolves with:* `ARCH-006` gpu-rendering-pipeline.
- **Mixer option-set placement** (part of Transfer vs its own set). *Resolves with:* `BRU-004` and a CS6 capture.
