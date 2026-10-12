# Liquify

- **Spec ID:** `FILT-090`
- **Status:** `Draft`
- **Parity tier:** `Core` — `Filter > Liquify` is in CS6 Standard; CS6 revamped the dialog (Basic/Advanced modes, GPU acceleration, last-mesh load) but the command itself predates CS6. Extended adds nothing filter-specific.
- **New in CS6:** `Changed` — CS6 adds **GPU/video-card acceleration**, a **Basic vs Advanced** interface split, **`Load Last Mesh`**, a maximum brush size of **15,000**, bracket-key brush resizing, an improved Mac cursor, and `Ctrl`/`Cmd` magnifier switching. The **Reconstruct** button's amount dialog and `Load Last Mesh` are CS6; the Reconstruct *mode* menus and the Displace/Amplitwist/Affine tool modes are marked **CS5-only** in the Help.
- **Depends on:** `FILT-001` filters-overview, `FILT-010` blur-filters (only for the shared filter dispatch/`Fade` model), `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-004` rust-qt-interop, `ARCH-005` threading-and-concurrency, `LAY-021` smart-filters, `LAY-020` smart-objects, `LAY-004` layer-masks, `IMG-005` bit-depth-and-conversion.

> Module, widget, and type names below are **design proposals** except where `## Rust module mapping` records the implementation (#348). Adobe's warp/reconstruct math and its `.msh` mesh format are closed; those parts are **behavioral parity only, algorithm TBD**. Facts are taken from the fetched CS6 Help PDF and the fetched CS6-era Adobe Help page unless marked *(inferred)*. Any **face-aware** content is **CC 2015.5**, not CS6, and is labelled as such.

## CS6 behavior

`Filter > Liquify` provides push, pull, rotate, reflect, pucker, and bloat distortions for any area of an image. Tools, options, and a preview live in the **Liquify dialog box**. In CS6, **select Advanced Mode to access more options**; the dialog otherwise opens in **Basic** mode. The filter "can be applied to 8-bits-per-channel or 16-bits per-channel images" — it is **not** on the Help's 32-bpc filter list. A type or shape layer must be **rasterized** first (or use the Type tool's Warp options instead).

### Distortion tools (sourced)

Several tools distort the brush area when the mouse button is held or the pointer is dragged. The distortion is **concentrated at the center of the brush** and **intensifies with repeated drags**.

- **Forward Warp** — pushes pixels forward as you drag. `Shift`-click with Warp, Push Left, or Mirror drags in a straight line from the previous click.
- **Reconstruct** — reverses an existing distortion as you hold and drag. `Shift`-click reconstructs in a straight line.
- **Twirl Clockwise** — rotates pixels clockwise; `Alt`/`Option` while dragging twirls counterclockwise.
- **Pucker** — moves pixels toward the brush center.
- **Bloat** — moves pixels away from the brush center.
- **Push Left** — pixels move left when you drag straight up (right when you drag down); dragging clockwise around an object enlarges it, counterclockwise shrinks it. `Alt`/`Option` reverses the mapping.
- **Mirror (CS5+)** — copies pixels into the brush area, mirroring perpendicular to the stroke (to the left of the stroke); `Alt`/`Option`-drag mirrors in the opposite direction. Best used with frozen areas; overlapping strokes create a water-reflection effect.
- **Turbulence (CS5+)** — smoothly scrambles pixels; useful for fire, clouds, and waves.

### Freeze / thaw mask

Frozen areas are protected from changes. They are painted with the **Freeze Mask** tool (drag; `Shift`-click freezes a straight line) or supplied from an existing **Selection, Layer Mask, Transparency, or Quick Mask** via the Mask Options pop-up. **Mask All** freezes every thawed area; **None** thaws everything; **Invert All** swaps frozen/thawed. **Show Mask** toggles the mask overlay and **Mask Color** changes its color. The Help explicitly notes that a **selection limits the preview and processing to the bounding rectangle of the selection** (for a rectangular marquee the preview and selected area are identical).

### Meshes

A mesh visualizes the displacement grid. **Show Mesh**, **Show Image**, mesh **size**, and mesh **color** control display. **Save Mesh** writes the distortion to a file; **Load Mesh** applies a saved mesh (scaled to fit if the image and mesh differ in size); **`Load Last Mesh`** (CS6-new) reapplies the most recent mesh. *(CC only, not CS6: meshes embedded automatically in the document for smart objects.)*

### Backdrop and view

**Show Backdrop**, **Use** (`Background` / `All Layers`), **Mode**, and **Opacity** preview other layers in front of or behind the active layer. **Only the active layer is distorted**, even when other layers are shown. With `Use = All Layers`, opacity 0 shows the target layer with the full filter effect.

### Reconstruct

After distorting, the user can reverse or reshape changes.

- **CS6:** click **Reconstruct** in the Reconstruct Options area, then specify an amount in the **Revert Reconstruction** dialog and click OK. **Restore All** removes all distortions, including in frozen areas.
- **Per-region:** freeze what must stay distorted, select the Reconstruct tool, drag (pixels move fastest at the brush center); a Reconstruct Mode menu appears in the tool options for **CS5 only**.
- **CS6 Reconstruct + `Alt`/`Option`** (CC-only enhancement per the Help): holding `Alt`/`Option` while dragging the Reconstruct tool across a warp **smooths** the warp instead of scaling it back. Shipped CS6 does **not** document this behaviour.
- **CS5-only reconstruction modes:** `Revert` (uniform scale-back, no smoothing), `Rigid` (preserve right angles at frozen/unfrozen edges), `Stiff` (weak-magnetic pull toward frozen distortions), `Smooth`, `Loose`, plus an intensity pop-up; and the tool's `Displace`, `Amplitwist`, `Affine` modes (copy the displacement/rotation/scaling/skew sampled at the click start point).

### Face-aware Liquify

CS6 has **no face-aware Liquify**. **Face-Aware Liquify** (automatic face detection with sliders for eyes, nose, mouth, face shape) shipped in **Photoshop CC 2015.5**. It is therefore a **post-CS6 extension** and is documented here only to record its absence from the parity baseline (see `## Open questions`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Liquify` | menu, dialog | — | 8-/16-bpc; rasterize type/shape first |
| Dialog — Basic / Advanced | mode toggle | — | CS6: Advanced exposes more options |
| Dialog — Zoom / Hand | tools | `Z` / `H` (`H`/space) | Alt-click Zoom = zoom out; hold space to pan |
| Dialog — Forward Warp | tool | `W` | pushes pixels |
| Dialog — Reconstruct | tool | `R` | reverses distortion; `Shift`-click straight line |
| Dialog — Twirl Clockwise | tool | `C` | `Alt`/`Option` = counterclockwise |
| Dialog — Pucker | tool | `S` | toward center; `Alt` reverses |
| Dialog — Bloat | tool | `B` | away from center; `Alt` reverses |
| Dialog — Push Left | tool | `O` | direction is relative to drag; `Alt` reverses |
| Dialog — Mirror | tool | `M` / `X` *(conflict)* | CS5 tool; CS6 new-feature list gives `X` |
| Dialog — Turbulence | tool | `T` | smooth pixel scramble |
| Dialog — Freeze Mask | tool | `F` | `Shift`-click = straight line |
| Dialog — Thaw Mask | tool | `D` | `Shift`-click = straight line |
| Dialog — Mask Options | panel | — | Selection / Layer Mask / Transparency / Quick Mask; Mask All / None / Invert All |
| Dialog — View Options | panel | — | Show Mesh, Mesh Size, Mesh Color, Show Image, Show Mask, Mask Color, Show Backdrop, Use, Mode, Opacity |
| Dialog — Save / Load / Load Last Mesh | buttons | `M` = Load Last Mesh | CS6-new `Load Last Mesh` |
| Dialog — Reconstruct / Restore All / Reset | buttons | `Alt`-click Reset = full reset | Reset reverts all distortions and options |
| Reconstruct Mode menu | combo | — | **CS5 only** |
| Brush Size / Density / Pressure / Rate / Turbulent Jitter | sliders | arrow keys | `Shift`+arrow = ×10; bracket keys resize brush |
| Dialog — interaction | modifier | `Ctrl`/`Cmd` = magnifier; Win `Alt`+right-drag = brush size; Mac `Ctrl`+`Alt`-drag = brush size | CS6-new |
| `Edit > Fade Liquify` | menu, dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode after commit |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | int px | 100 (CS6 capture, #348) | 1–**15,000** (CS6 max) | Bracket keys, `Alt`+right-drag (Win), `Ctrl`+`Alt`-drag (Mac) |
| Brush Density | int % | *(unverified)* | 0–100 | Feathers the brush edge; effect strongest at center |
| Brush Pressure | int % | 100 (CS6 capture, #348) | 0–100 | Speed at which distortions accumulate while dragging |
| Brush Rate | int % | *(unverified)* | 0–100 | Speed of application while a tool is held stationary |
| Turbulent Jitter | int % | *(unverified)* | 0–100 | CS5+; tightness of the Turbulence scramble |
| Reconstruct Mode | enum | — | Revert / Rigid / Stiff / Smooth / Loose | **CS5 only** |
| Reconstruct tool mode | enum | — | Displace / Amplitwist / Affine | **CS5 only** |
| Stylus Pressure | bool | on when a tablet is present | on / off | Brush pressure = stylus pressure × Brush Pressure |
| Mask source | enum | — | Selection / Layer Mask / Transparency / Quick Mask | Each of the five mask-option icons |
| Mask combine | enum | Replace | Replace / Add To / Subtract From / Intersect With / Invert | Applied to the frozen area |
| Show Mesh | bool | off | on / off | Mesh overlay |
| Mesh Size | enum | *(unverified)* | small / medium / large | Display only |
| Mesh Color | swatch | *(unverified)* | color picker | Display only |
| Show Image | bool | on | on / off | With Show Mesh only, shows mesh alone |
| Show Mask | bool | off | on / off | Frozen-area overlay |
| Mask Color | swatch | *(unverified)* | color picker | Overlay color |
| Show Backdrop | bool | off | on / off | Preview other layers |
| Use (backdrop) | enum | Background | Background / All Layers | Only the active layer is distorted |
| Backdrop Mode | enum | *(unverified)* | blend-mode list | Combines backdrop with target in preview |
| Backdrop Opacity | int % | *(unverified)* | 0–100 | 0 + All Layers shows the full filter effect |
| Reconstruction amount | int % | *(unverified)* | 0–100 | CS6 Revert Reconstruction dialog |

## Algorithms & pipeline

Behavioral parity only. Adobe has not published the Liquify kernel or mesh format; the model below is the standard inverse-distance / displacement-field construction used by independent reimplementations and is marked *(inferred)* except where the Help states the observable rule.

**Data model.** Liquify is a **mesh warp**: a coarse lattice over the image holds a 2-D displacement per grid node, and pixels are resampled by bilinear/bicubic interpolation of the displaced lattice. The dialog preview and the committed result share the same lattice so "what you see" is "what you get."

```text
mesh: H x W lattice of vectors (u,v), initially (0,0)
for each drag sample p with brush falloff w(p) in [0,1]:
    # density shapes w; pressure and rate shape the accumulation gain
    mesh[p] += k * w(p) * d(p)
    # rate controls how often this accrues while p is stationary
```

Tool displacement fields *(inferred)*:

- **Forward Warp** — `d = drag_delta`, i.e. pixels are advected along the pointer path.
- **Twirl** — `d = R(±θ) · (p − center) − (p − center)`, a rotation about the brush center.
- **Pucker / Bloat** — `d = ∓ (p − center) · g(‖p−center‖)`, radial contraction/expansion.
- **Push Left** — `d` is the drag direction rotated **90°** (the Help's "up-drag moves left").
- **Mirror** — `d` copies the displacement sampled on the reflected side of the stroke axis across the brush.
- **Turbulence** — `d` is a smooth procedural noise (value/Perlin/simplex-like) field; **Turbulent Jitter** sets the noise frequency/scale *(inferred)*.

**Brush falloff.** The Help describes the effect as strongest at the center and lighter at the edge; mathematically `w(p)` is a radial falloff whose feather width is **Brush Density**. **Brush Pressure** scales the per-event gain; **Brush Rate** is the timer-based repetition when a tool is stationary (Twirl, Pucker, Bloat).

**Freeze mask.** An 8-bit mask: 0 = editable, 255 = frozen; painted with a brush and combined with an input channel via Replace/Add/Subtract/Intersect/Invert. Frozen texels are excluded from the displacement integration.

**Reconstruct** *(inferred)*. Reconstruction is a relaxation of the unfrozen displacement field toward either zero or the frozen field:

- `Revert` — scale every unfrozen displacement toward 0, uniformly, no smoothing.
- `Rigid` — solve so that frozen/unfrozen boundaries keep right angles (restores approximate original appearance).
- `Stiff` — unfrozen cells are pulled toward nearby frozen distortions with distance-decaying strength.
- `Smooth` / `Loose` — propagate frozen displacement through unfrozen cells with increasing continuity.
- `Displace` / `Amplitwist` / `Affine` — fit a similarity/affine transform at the start point and re-apply it along the stroke.

The Help's "amount" in the CS6 Revert Reconstruction dialog is a blend factor between the current and reconstructed fields.

**GPU and bit depth.** CS6 added GPU video-card acceleration for Liquify, and Adobe's GPU FAQ lists Liquify as accelerated (not required). Bit depth is 8 or 16 bpc; resample in the working space and keep intermediate displacement/premultiplied values at higher precision to avoid 8-bit banding.

## Rust module mapping

**Implemented (#348, OpenSpec change `add-liquify`).** `pictura_filters::distort::liquify` ports photocraft's displacement field (MIT OR Apache-2.0): `LiquifyMesh` holds an inverse offset per node of a 2 / 4 / 8 px grid (by layer area) plus a freeze mask; `LiquifyBrush` / `LiquifyTool` dab it (each tool reads the current output at `q = M(p)`, so `d' = q + d(q) − p`), with Kooka's own Mirror and Turbulence; `reconstruct(amount)`, `restore_all`, Mask All / None / Invert All and `scaled_to` (Load Mesh's scale-to-fit). `Filter::Liquify { mesh }` resamples through `liquify` (Catmull-Rom mesh and source; an identity mesh is bit-exact) and `LiquifyProxy` renders the dialog preview. The app keeps one dialog session and the kept meshes in `filter_tools/liquify.rs`; the `liquify` kind's one slot is a kept mesh id, so OK commits one filter state and Last Filter replays it. The dialog is `LiquifyDialog` + `LiquifyCanvas` (C++). Tool strengths, Density/Rate/Jitter defaults (50 / 80 / 50) and Mirror/Turbulence are approximations. Not built: `.msh` Save/Load Mesh, mask sources (Selection/Layer Mask/Transparency/Quick Mask) and combine modes, backdrop, mesh overlay, CS5 reconstruct modes, a live Reconstruct preview, and the GPU path.

The proposals below remain the long-form design. The whole-image interactive warp is a distinct pipeline from the tile-based paint tools, but reuses the document tile store and history model.

- `pictura_filters::liquify` — `LiquifySession` owning a `WarpMesh`, an input snapshot, and the option set; methods `apply_tool(ToolEvent)`, `reconstruct(amount)`, `restore_all()`, `to_displacement()`, `commit(&mut Document)`.
- `pictura_filters::liquify::mesh` — `WarpMesh { dim: (u16, u16), nodes: Vec<Vec2f> }`, `sample_bilinear(&self, p) -> Vec2f`, `save_msh`, `load_msh` (format TBD), `resize_to(image_size)` for `Load Mesh` scaling.
- `pictura_filters::liquify::tools` — `enum LiquifyTool { ForwardWarp, Reconstruct, Twirl, Pucker, Bloat, PushLeft, Mirror, Turbulence }`, `fn displacement(tool, p, center, drag, opts) -> Vec2f`.
- `pictura_filters::liquify::mask` — `FreezeMask { data: MaskBuffer, combine: MaskCombine }`, `combine_with(&ChannelView)`.
- `pictura_filters::liquify::reconstruct` — `enum ReconstructMode { Revert, Rigid, Stiff, Smooth, Loose, Displace, Amplitwist, Affine }`, `fn relax(field: &mut DisplacementField, frozen: &MaskBuffer, mode, amount)`.
- `pictura_render::liquify` — optional GPU warp: upload the mesh as a storage buffer, sample the source in a fragment/compute pass with bicubic interpolation; CPU path is the correctness reference.
- `pictura_render::mask_overlay` — mask/mesh/backdrop overlay compositing for the preview.

Crossing types: `Vec2f`, `WarpMesh`, `DisplacementField`, `MaskBuffer`, `LiquifyParams` (serializable option set), `FilterResult { tiles, fade_state }`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LiquifyDialog` | `QDialog` | Owns the session; Basic/Advanced page switch; OK/Cancel/Reset (Alt-click Reset); Reconstruct/Restore All/Load Last Mesh |
| `LiquifyToolPalette` | `QToolBar` | Tool buttons + `QShortcut`s (`W R C S B O M T F D`) |
| `LiquifyOptionsPanel` | `QWidget` | Size/Density/Pressure/Rate/Jitter sliders; stylus toggle; Reconstruct Mode combo (CS5) |
| `LiquifyMaskPanel` | `QWidget` | Mask source, combine mode, Mask All/None/Invert All, mask color |
| `LiquifyViewPanel` | `QWidget` | Show Mesh/Image/Mask/Backdrop, mesh size/color, Use, Mode, Opacity |
| `LiquifyCanvas` | `QRhiWidget` (preview) + `QGraphicsView` (overlay) | Pointer/tablet input, live warp preview, mesh/mask/backdrop overlays, zoom/hand |
| `MeshModel` | `QAbstractTableModel` *(proposal)* | Optional read-only diagnostics of lattice nodes |

Widgets over QML: this is a modal, canvas-centric desktop dialog with precise pointer capture and tablet events; `QWidget`/`QRhiWidget` matches the rest of the app frame (`ARCH-003`) and avoids a QML/`QGraphicsView` event-routing split for a modal editor.

## Data-model impact

- **Destructive edit** with an **optional mesh**: `OK` produces one history state; the pre-filter tile/image snapshot is retained for undo (`ARCH-009`). Live previews inside the dialog are **not** history states.
- **Mesh persistence:** the CS6 Help documents saving meshes as separate files (`Save Mesh`/`Load Mesh`/`Load Last Mesh`), not as a PSD asset. Store the mesh in the filter record's `params_blob` so a smart-filter re-edit can restore it (`LAY-021`); keep `Load Last Mesh` as session state (`11-cross-cutting/preference-storage.md`).
- **Smart filter (CC only):** shipped CS6 does not let Liquify be a smart filter; a CC/extension build that does must add the mesh to the Smart Object's embedded data, which even compressed increases the file size.
- **Fade:** `Edit > Fade Liquify` stores opacity + mode for the last filter commit.
- **No new document nodes**; the freeze mask is transient dialog state unless the user supplies a channel/selection.

## Edge cases

- **32-bpc documents** — Liquify is **not** on the CS6 32-bpc filter list; grey out the menu (or offer a documented conversion). 16-bpc is explicitly supported.
- **Modes** — Bitmap and Indexed cannot take filters at all; whether CMYK/Lab are supported for Liquify is not stated in the fetched Help (see `## Open questions`).
- **Type / shape layer** — must be rasterized; otherwise disable with an explanatory state.
- **Selection present** — the preview and processing are restricted to the selection's **bounding rectangle**; a rectangular marquee makes "Selection" mask mode a no-op.
- **Everything frozen** — every tool becomes a no-op; do not create a history state.
- **Mesh size mismatch** — `Load Mesh` scales the mesh to the image; define and test the scaling convention (corner-anchored, image-relative).
- **Reset** — `Alt`-click Reset restores pixels **and** resets option defaults; `Restore All` removes distortions but keeps options.
- **1-px and empty documents** — a 1×1 image has no meaningful warp; empty documents are rejected at open.
- **Huge (PSB) documents** — the warp mesh and preview must be memory-bounded (lattice, not per-pixel); process in tiles with a coarse mesh.
- **GPU unavailable** — CS6 documents acceleration, not a hard requirement for Liquify; fall back to the CPU path (unlike Oil Paint / Blur Gallery, which require a supported card).
- **Undo/redo** — one atomic state per commit; a cancel discards the session without touching history.
- **Stylus** — `Stylus Pressure` off must ignore pressure entirely (pressure = 1).

## Parity acceptance criteria

1. Given a flat image, a single Forward Warp drag moves pixels in the drag direction, with effect falling off toward the brush edge and no change outside the brush footprint.
2. Given a uniform gray image with a known center, `Twirl` produces an angle of rotation that grows monotonically with hold time; `Alt`-dragging reverses the sign.
3. Given a radial test pattern, `Pucker` decreases mean radius within the brush and `Bloat` increases it; `Alt` inverts each.
4. Given a vertical drag, `Push Left` moves pixels to the left (and to the right for a downward drag) per the Help's description.
5. Given a frozen region, no distortion tool changes pixels where the freeze mask is 255; `Invert All` swaps the protected set.
6. Given a prior distortion, `Restore All` returns the image to the pre-filter state **bit-exactly** and `Reconstruct` with amount 100 equals the original within resampling tolerance; amount 0 is a no-op.
7. Given a saved mesh applied to a same-size image, the displacement field matches the saved session; on a differently sized image the mesh is scaled to fit.
8. Given a selection, processing is confined to the selection's bounding rectangle.
9. Given an 8-bpc and a 16-bpc document, commits are reversible via one history state per commit.
10. Given no compatible GPU, the CPU path yields the same acceptance results within latency tolerance.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded to `/tmp`, `pdftotext`-extracted). Established: the tool/dialog overview (push, pull, rotate, reflect, pucker, and bloat distortions); Advanced Mode; 8-/16-bpc only; Forward Warp / Reconstruct / Twirl / Pucker / Bloat / Push Left / Mirror (CS5) / Turbulence (CS5) behaviour and `Alt` reversals; `Shift`-click straight lines; Distortion Tool Options (Brush Size, Density, Pressure, Rate, Turbulent Jitter, Reconstruct Mode CS5, Stylus Pressure); Freeze/Thaw mask and Mask Options (Replace/Add/Subtract/Intersect/Invert, Mask All, None, Invert All, Show Mask, Mask Color); the selection bounding-box processing rule; meshes (Show Mesh/Image, Save/Load Mesh scaled to fit, **`Load Last Mesh` CS6**); backdrop (Use, Mode, Opacity, with only the active layer distorted); reconstruction (CS6 Reconstruct button + Revert Reconstruction amount, Restore All, CS5 modes Revert/Rigid/Stiff/Smooth/Loose + intensity, CS5 Displace/Amplitwist/Affine); CS6 What's-New Liquify list (GPU acceleration, Basic/Advanced, Load Last Mesh, **max brush 15,000**, bracket resize, Mac cursor, `Ctrl`/`Cmd` magnifier, `Alt`+right-drag brush size); the "Keys for Liquify" table; the 16-bpc filter list includes Liquify and the 32-bpc list excludes it.
- `https://web.archive.org/web/20121116114821id_/http://helpx.adobe.com/photoshop/using/blur-gallery.html` — CS6-era Adobe Help snapshot (wayback) of the sibling Blur Gallery page; confirmed the CS6 Help era and wording style. Not used for Liquify facts.
- `https://topic.alibabacloud.com/a/photoshop-cs6-gpu-faq_8_8_10184243.html` — mirror of the **Photoshop CS6 GPU FAQ**. Established: Mercury Graphics Engine uses OpenGL + OpenCL (not CUDA); Liquify is **accelerated** by a compatible card (not gated); Adaptive Wide Angle and Oil Paint **require** a compatible card.
- `https://help.adobe.com/...` 403 and `https://helpx.adobe.com/photoshop/using/liquify-filter.html` 403 from this environment.
- `https://the-digital-photography-school.com/new-oil-paint-filter-in-photoshop-cs6` etc. — not used for Liquify.
- `https://www.photoshopessentials.com/photo-editing/use-face-aware-liquify-photoshop-cc` and `https://gregbenzphotography.com/photography-tips/face-aware-liquify-photoshop-cc-2015-5` — community sources locating **Face-Aware Liquify** at **CC 2015.5**, i.e. **after** CS6. Secondary/community.
- SearXNG meta-search (queries: "Photoshop CS6 Face-Aware Liquify introduced version CC 2015", "Photoshop CS6 GPU FAQ") — used to locate the above; no facts asserted from snippets alone.

Not parsed in this pass: `helpx.adobe.com` live pages (HTTP 403 from this environment); the `.msh` mesh format (proprietary, not published).

## Open questions

- **All Liquify slider defaults** (Size, Density, Pressure, Rate, Jitter, mask/mesh/backdrop colors and sizes). Not stated in the fetched Help. Resolves with: a first-run CS6 dialog capture.
- **Mirror tool shortcut conflict.** The "Keys for Liquify" table gives `M` = Mirror, while the CS6 new-feature shortcut list gives `M` = Load Last Mesh and `X` = Mirror. Resolves with: CS6 keyboard-shortcut editor / a build capture. The spec assumes **CS6 uses `X` for Mirror** so `M` can load the mesh, but this is unresolved.
- **CS6 reconstruct semantics.** Whether shipped CS6 retains the CS5 Reconstruct **mode** menus and Displace/Amplitwist/Affine or replaced them with the amount dialog. Resolves with: a CS6 dialog capture.
- **Exact 8/16-bit mode support** (RGB/Grayscale/CMYK/Lab). The Help says 8/16-bpc but does not enumerate modes for Liquify. Resolves with: a mode test in CS6.
- **Mesh file format and scaling rule.** Closed/unpublished. Resolves with: an interoperability target or a documented replacement format; behavior is **parity only**.
- **Face-Aware Liquify parity decision.** Confirmed **CC 2015.5, not CS6**. Decide whether to implement it as a clearly-labelled post-CS6 extension; it must not appear in the CS6 parity baseline.
- **Performance budget.** Interactive warp latency targets belong to `01-architecture/performance-targets.md`; CS6 accelerated this on the GPU, so the CPU fallback budget is a product decision.
- **CC-only Reconstruct smoothing.** The `Alt`/`Option`-drag "smooth rather than scale back" behaviour is documented as **CC**, not CS6; confirm before adding.
