# Rotate View Tool

- **Spec ID:** `TOOL-043`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the tool exists in CS6 but was **not** introduced in CS5. Community sources state it shipped in **CS4** (a "Photoshop CS4: Rotate View Tool" video from November 2008 and two tutorials both place its introduction in CS4). The brief for this spec said "new in CS5"; the fetched sources contradict that (see `## Open questions`). CS6 carries it forward unchanged.
- **Depends on:** `TOOL-042` hand-and-zoom, `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/qt6-ui-design.md`, `02-ui-ux/application-frame.md`, `07-color-painting/brush-engine.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

The Rotate View tool rotates the **canvas** non-destructively; "it does not
transform the image." The CS6 Help frames the use case as easier painting or
drawing. The tool is required to have **OpenGL**: "(OpenGL is required.)" If the
tool isn't visible in the toolbox, hold down the Hand tool to reveal it; the
single-letter shortcut is `R`.

- **Drag** in the image to rotate. "A compass will indicate north in the image,
  regardless of the current canvas angle" — the compass's red marker points to
  the true top of the image as the canvas turns.
- **Rotation Angle** — the options bar has a numeric field; type an angle.
- **Set Angle of Rotation** — a circular dial in the options bar; click or drag
  it.
- **Reset View** — restores the canvas to its original angle.
- **Mac trackpad rotate gestures** — the Help notes "You can also use rotate
  gestures on MacBook computers with multi-touch trackpads," and documents
  disabling them via `Photoshop > Preferences > Interface > Enable Gestures`
  (Mac-only).
- The tool's purpose is stated in the Brush-tool topic as well: "The Rotation
  tool rotates the canvas, which can facilitate easier painting."

The CS6 Help does **not** document **Rotate All Windows** or
`Window > Arrange > Match Rotation`; those appear in a CC-era tutorial that also
claims CS6 compatibility, so they are treated as present-in-CC / unverified-in-CS6
(see `## Open questions`). Community sources also add **`Esc`** as a Reset View
shortcut, **`Shift` while dragging = 15° steps**, a **Scrubby Slider** on the
"Rotation Angle" label (1° per unit, 10° with `Shift`), and a **spring-loaded**
`R` (hold to rotate, release to return to the previous tool).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox navigation slot | Tool (fly-out group with Hand) | `R` | Visible tool = last used; hold Hand to reveal |
| Options bar | Tool options | `R` | Rotation Angle field, Set Angle of Rotation dial, Reset View; Rotate All Windows *(unverified in CS6)* |
| Rotation Angle field | Scrubby numeric | n/a | Type degrees or scrub; `Shift` = 10° steps *(CC-era community)* |
| Shift + drag | Modifier | `Shift` | 15° snap *(CC-era community)* |
| Reset View | Button / `Esc` | `Esc` *(community)* | Restores 0° |
| Spring-loaded tool | Modifier | hold `R` | Switch, rotate, release; options bar inaccessible while held *(community)* |
| Mac trackpad gesture | Gesture | n/a | `Preferences > Interface > Enable Gestures` to disable |
| Window > Arrange > Match Rotation | Menu | n/a | *Unverified in CS6; present in CC* |
| Preferences > Performance | Pane | n/a | Enable OpenGL Drawing gates the tool |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Rotation Angle | float degrees | 0 | -180 … 180 *(inferred)* | Options-bar field/dial; exact bound unverified |
| Rotation drag | continuous | — | free; `Shift` = 15° steps | Compass overlay during drag |
| Set Angle of Rotation | circular dial | 0 | continuous | Options bar |
| Scrubby Slider step | float degrees | 1 | 1 (10 with `Shift`) | Community source, CC-era |
| Reset View | command | — | — | Also `Esc` *(community)* |
| Rotate All Windows | bool | Off | on / off | *Unverified in CS6* |
| Enable Gestures (Mac) | bool | On | on / off | Mac-only preference |

## Algorithms & pipeline

Rotate View is a **view-only** operation. It adds a rotation term to the
viewport transform of `TOOL-042`:

```text
ViewportTransform {
    scale: f32,
    translation: Vec2,
    rotation_deg: f32,   // this tool's state
}
```

- The document's tiles are never resampled. The GPU compositor transforms the
  composited output (or samples the tile set) by the rotation when drawing to the
  canvas.
- **Transform order** matters: rotation must be applied about the view centre (or
  the drag pivot), then translation, so that pan and zoom remain intuitive when
  the canvas is rotated *(inferred)*.
- **Non-100% quality**: the rotated canvas is resampled on the GPU; the exact
  filtering (bilinear vs bicubic) is *(inferred)* and should match the zoom
  policy chosen in `ARCH-006`.
- **Compass and dial** are overlay UI, drawn in window space, not part of the
  document.
- **Rulers / pixel grid** rotate with the canvas in CS6 *(inferred)*; verify.
- **GPU requirement**: the tool is unavailable (greyed) without a usable GPU,
  per the OpenGL requirement. The CPU fallback policy — disable the tool versus
  emulate rotation in software — is a decision; the CS6 wording implies disable.

## Rust module mapping

Proposals; rotation extends the navigation substrate rather than adding a new
subsystem.

- `pictura_render::viewport` — `ViewportTransform::rotation_deg` (already
  provided for by `TOOL-042`); `set_rotation`, `rotate_by`, `reset_rotation`,
  `snap(angle, step)`.
- `pictura_tools::navigate::rotate` — `RotateViewTool`, `RotateOptions
  { angle_deg, rotate_all_windows, snap_step }`, `CompassState`.
- `pictura_tools::navigate::group` — `rotate_all(angle)` across windows
  (if parity confirms it) and `match_rotation`.

Crossing types: `AngleDeg(f32)`; no document types cross the boundary, which is
the point — the document is untouched.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `RotateViewOptionsWidget` | `QWidget` (options bar) | Rotation Angle scrub field, dial (`QDial`-like custom), Reset View, Rotate All Windows toggle |
| `CanvasView` | `QGraphicsView` | Drag-to-rotate hit handling; compass overlay item |
| `CanvasWidget` | `QRhiWidget` | Applies the combined transform when rendering |
| `CompassOverlay` | `QGraphicsItem` | Draws the compass and north marker in view space |
| `GestureRouter` | `QObject` | Routes `QNativeGestureEvent` rotate gestures (trackpad) to the viewport |

The rotate gesture path is Qt's `QNativeGestureEvent`
(`beginNativeGesture`/`nativeGesture`, `Qt::RotateNativeGesture`) on X11 and
Wayland where the compositor supplies it; where it does not, the tool remains
fully usable via drag and the Rotation Angle field.

## Data-model impact

- **No document mutation** and **no history state**. Rotation is not undoable in
  CS6 *(inferred)*; it must never be recorded in `ARCH-009`.
- **View state only**: `rotation_deg` is per-window `ViewportState`, alongside
  zoom and scroll (`TOOL-042`), saved in the workspace/session if CS6 persists it
  *(unverified)*. It is not document content.
- **Persistence question**: whether CS6 stores canvas rotation in the PSD/PSB or
  only in the session/workspace is not established. If it is stored, a serialized
  field is needed; if not, none.
- **Preferences**: the Mac `Enable Gestures` toggle maps to a Linux
  gesture-enable preference (`11-cross-cutting/preference-storage.md`).

## Edge cases

- **GPU unavailable** — CS6 requires OpenGL; the tool should be disabled or
  hidden rather than silently doing nothing. The exact CS6 fallback is unverified.
- **Combined transform** — rotation interacts with zoom (anchoring and rounding)
  and pan; must not drift over repeated rotate/zoom cycles.
- **Snap angles** — `Shift`-drag at 15° and scrubby 10° must land on exact
  multiples; `Reset View` returns to exactly 0°.
- **Trackpad gestures** — on Linux, gesture availability depends on the
  compositor/`QNativeGestureEvent`; avoid double-applying a gesture and a drag.
- **Multi-window / multiple views of one document** — rotation is per window;
  closing a window must not alter other windows' rotation.
- **Tablet input** — rotation dabs and the compass must remain correct under
  canvas rotation; the tool reports document-space coordinates, not window space.
- **Extreme angles / 1-px / huge documents** — the transform must stay finite and
  the tile renderer must not allocate a rotated full-resolution buffer.
- **Mid-rotation undo** — irrelevant: view changes are not in history, and
  `Ctrl+Z` during a rotation does nothing.

## Parity acceptance criteria

- Given a document, dragging with the Rotate View tool rotates the displayed
  canvas clockwise or counterclockwise without changing a single stored pixel
  (save and compare checksums).
- Given the canvas is rotated, the compass red marker always points to the
  document's top edge.
- Given a value typed into Rotation Angle, the canvas rotates to that exact
  angle; the dial and the field stay in sync.
- Given `Reset View` (or `Esc`), the angle returns to 0°.
- Given `Shift` is held while dragging, the angle changes in 15° steps.
- Given OpenGL/GPU is unavailable, the Rotate View tool is disabled/hidden and
  the application remains stable.
- Given a rotated canvas, zooming and panning keep the point under the cursor /
  drag anchor stable within tolerance.
- Given a rotation, save the document, reopen it, and the pixels are identical to
  the pre-rotation save (rotation is non-destructive).
- Given a rotate gesture on a supported trackpad, the canvas rotates; disabling
  gestures stops it.
- Given multiple windows, rotating one does not rotate the others unless
  Rotate All Windows is on (if that option is confirmed for CS6).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  "Use the Rotate View tool to rotate the canvas non-destructively; it does not
  transform the image"; "(OpenGL is required.)"; select from the toolbox, holding
  the Hand tool if hidden; drag with the compass indicating north regardless of
  canvas angle; the Rotation Angle field; the circular Set Angle of Rotation
  control; Reset View; Mac multi-touch rotate gestures and the `Enable Gestures`
  Interface preference; the `R` toolbox shortcut; and that the Brush-tool topic
  calls it "the Rotation tool … rotates the canvas" for easier painting.
- `https://www.photoshopessentials.com/basics/photoshop-rotate-view-tool` —
  community tutorial marked "For Photoshop CC and CS6": compass behaviour,
  `Shift` = 15° drag steps, Scrubby Slider (1°/10°), spring-loaded `R`, `Esc`
  reset, **Rotate All Windows**, and `Window > Arrange > Match Rotation`.
  Secondary source; the last two are CC-era and unverified for CS6.
- `https://skillforge.com/skillforge-blog/photoshop-tips-and-tricks-rotate-view-tool`
  — community post: "It was introduced with Photoshop CS4"; `R` shortcut or
  hold the Hand tool; compass during drag; Rotation Angle in the options bar;
  Reset View. Secondary source.
- `https://glensmith.co.uk/photoshop/rotate-tool` — community tutorial:
  Rotation Angle field, Image Rotation dial, Reset View, Rotate All Windows,
  `R` shortcut and `Esc` reset. Secondary source, CC-era.
- SearXNG meta-search (queries: "Photoshop Rotate View tool introduced CS4 or
  CS5 rotatable canvas"; "Photoshop Rotate View tool CS6") — used to locate the
  above; the CS4 Vimeo "Photoshop CS4: Rotate View Tool Overview" (Nov 2008) was
  the strongest dated signal but was not fetched.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Introduction version: CS4 or CS5?** The brief says CS5; a dated CS4 video and
  two secondary tutorials say CS4. This affects the `New in CS5`/`New in CS6`
  traceability row. *Resolves with:* a CS4 and CS5 "What's New" page or the
  version-history section of the CS6 Help PDF.
- **Does CS6 have Rotate All Windows and Match Rotation?** The CS6 Help does not
  mention them; a CC tutorial places them at/after that era. *Resolves with:* a
  CS6 options-bar capture and the `Window > Arrange` menu in CS6.
- **OpenGL-unavailable behaviour.** Whether CS6 greys out the tool, hides it, or
  errors is not stated; only "OpenGL is required." *Resolves with:* a CS6 run on
  an unsupported GPU.
- **Rotation Angle range and step.** The field's min/max and default increment
  are not documented. *Resolves with:* a CS6 field inspection.
- **Persistence of canvas rotation.** Whether rotation is saved in the PSD/PSB,
  the workspace, or only the open session is unverified. *Resolves with:* a CS6
  save/reopen/restart test; feeds `ARCH-011` file-formats if it is stored.
- **Rulers, grid, and guides under rotation.** Whether they rotate with the
  canvas and how they report coordinates is unverified. *Resolves with:* a CS6
  screenshot/test.
- **GPU filtering quality.** The resampling filter used for the rotated canvas is
  not documented. *Resolves with:* `ARCH-006`'s rendering decision and a
  comparison against CS6 output.
- **Linux gesture parity.** Which Wayland/X11 compositors deliver rotate gestures
  and how they map to the Mac behaviour is untested. *Resolves with:* a Qt6
  gesture prototype on both display servers.
