# Clone Source Panel

- **Spec ID:** `PAN-025`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Clone Source panel and its transform/overlay controls are in both CS6 editions for Clone Stamp and Healing Brush. The video/animation frame-relationship controls are `Extended-only`.
- **New in CS6:** `No` — the panel and its controls are carried from CS5. CS6 documents it unchanged for Clone Stamp and Healing Brush; the Extended frame-relationship options remain.
- **Depends on:** `03-tools/clone-stamp-and-pattern-stamp.md` (`TOOL-030` — Clone Stamp/Pattern Stamp tool semantics, sampling model, and the same panel contract; this spec is the **panel UI**), `03-tools/healing-brushes.md` (Healing Brush shares the panel), `02-ui-ux/toolbox-and-options-bar.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/qt6-ui-design.md` (`ARCH-003`), `01-architecture/gpu-rendering-pipeline.md` (`ARCH-006`).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. The panel overlaps
> `03-tools/clone-stamp-and-pattern-stamp.md`; this spec focuses on the panel's
> own surface and state.

## CS6 behavior

The **Clone Source panel** (`Window > Clone Source`) *"has options for the Clone Stamp tools or Healing Brush tools."* It lets the user keep several sampling sources and switch between them without resampling, preview the source as an overlay, and scale or rotate it to match the destination.

### Five sample sources

You can define up to five different sample sources and switch quickly between them without resampling each time you change source. Five **source buttons** select the active slot. `Alt`/`Option`-click in any open document window sets the sampling point for the selected slot; clicking a different button and sampling again defines that slot. The panel keeps the sampled sources until the document is closed. Sources can come from the current document or any other open document (subject to the tool's color-mode rules in `TOOL-030`).

### Offset

Each source has an **Offset** x/y pixel value that places the sampled source at a specified position relative to the sampling point. Select the source, then enter the x and y values.

### Scale and rotation

- **W (width)** and **H (height)** percentage fields scale the source; **proportions are constrained by default**, with a **Maintain Aspect Ratio** button to toggle constraint or adjust dimensions independently.
- **Rotate** is entered in degrees (or scrubbed on the **Rotate The Clone Source** icon).
- **Reset Transform** restores the source to its original size and orientation.

### Flip

**Flip Horizontal** and **Flip Vertical** buttons reverse the source direction (the Help notes this is useful for mirroring features such as eyes).

### Overlay

Select **Show Overlay** to preview the source on canvas. The overlay options are:

- **Auto Hide** — hide the overlay while painting.
- **Clipped** — clip the overlay to the brush size.
- **Opacity** — percentage.
- **Blend mode** — **Normal / Darken / Lighten / Difference** (pop-up at the bottom of the panel).
- **Invert** — invert the overlay colors.

Documented alignment trick: **Opacity 50% + Invert + unclipped** makes identical source/destination areas appear **solid gray** when aligned, which helps register a clone.

**Move Source Overlay.** Hold `Alt+Shift` (`Option+Shift`) while painting with the Clone Stamp to temporarily switch to the **Move Source Overlay** tool; drag to reposition the overlay.

### Keyboard

The CS6 key tables list: **Show Clone Source (overlays image)** `Alt+Shift`/`Opt+Shift`; **Nudge Clone Source** `Alt+Shift+arrow`; **Rotate Clone Source** `Alt+Shift+<`/`>`.

### Extended-only frame relationship

*(Photoshop Extended)* For timeline-based animations the panel also provides options that specify the frame relationship between the sampled source video/animation frame and the target video/animation frame. Out of core parity (`OVR-003`, `docs/02-ui-ux/panels/timeline-panel.md`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Clone Source` | Menu → dock panel | — | The panel |
| Panel — source buttons | Five toggle buttons | — | Active sampling slot (1–5) |
| Canvas | `Alt`/`Option`-click | — | Set the sampling point |
| Panel — W / H | Numeric fields | — | Scale percent; Maintain Aspect Ratio |
| Panel — Rotate | Numeric / scrub icon | `Alt+Shift+<`/`>` | Degrees |
| Panel — Flip H / Flip V | Toggle buttons | — | Mirror source |
| Panel — Offset X / Y | Numeric fields | — | Source placement |
| Panel — Reset Transform | Button | — | Restore size/orientation |
| Panel — Show Overlay | Checkbox | `Alt+Shift` (overlay display) | Source preview |
| Panel — Auto Hide / Clipped / Opacity / mode / Invert | Controls | — | Overlay appearance |
| Canvas overlay | Drag with Move Source Overlay | Hold `Alt+Shift` | Reposition overlay |
| Panel — frame controls | Extended | — | Video/animation frame relationship |
| Options bar (Clone/Healing) | Bar | — | Aligned, Sample, brush, mode, opacity, flow (`TOOL-030`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Clone source slot | enum | 1 | 1–5 | Persists until the document closes |
| Sampling point | point | none | any open image | Set by `Alt`/`Option`-click |
| Source Width (W) | percent | 100 | > 0 | Proportions constrained by default |
| Source Height (H) | percent | 100 | > 0 | Independently adjustable when unconstrained |
| Maintain Aspect Ratio | bool | on | on/off | Toggle constraint |
| Rotation | degrees | 0 | any (signed) | Scrub or type |
| Flip Horizontal / Vertical | bool | off | on/off | Mirrors the source |
| Offset X / Y | pixels | 0 | any (signed) | Source placement |
| Show Overlay | bool | off | on/off | Source preview |
| Auto Hide | bool | off | on/off | Hide while painting |
| Clipped | bool | off | on/off | Clip overlay to brush size |
| Overlay Opacity | percent | 100 *(not stated; 100 assumed)* | 0–100 | Overlay only |
| Overlay blend mode | enum | Normal | Normal / Darken / Lighten / Difference | Overlay only |
| Invert | bool | off | on/off | Invert overlay colors |
| Reset Transform | command | — | — | Restores 100%/100%/0° |

*(Ranges for the overlay/transform fields beyond those stated are inferred; the CS6 text gives meaning, not slider maxima.)*

## Algorithms & pipeline

The panel is a **controller + overlay renderer** over the clone source session; the sampling/painting pipeline is `TOOL-030`.

1. **Slot state** — a `CloneSourceSet` of five `CloneSourceSlot { anchor, transform, offset, overlay }` values. Selecting a slot rebinds the tool's active source; each slot persists until the document closes (never serialized).
2. **Source→destination mapping** — the panel edits an affine transform `T` (scale Sx/Sy, rotation θ, flips) plus an explicit `(offset_x, offset_y)`. For a destination dab centre `d`, the sampled source point is `src = T⁻¹(d − offset) − anchor_delta` per the aligned/non-aligned rule in `TOOL-030`.
3. **Overlay render** — draw the transformed source under/over the canvas with the overlay blend mode, opacity, optional clip-to-brush and invert; `Auto Hide` suppresses it during a stroke. The 50%+Invert effect falls out of the standard `Difference`-style math when source and destination match.
4. **Move Source Overlay** — `Alt+Shift` swaps the tool to a transient overlay-drag mode that edits the offset/transform without sampling pixels.
5. **Reset Transform** — restore an identity transform for the selected slot.

Because the mapping is a pure affine transform on the sample view, the panel can preview cheaply and the GPU overlay (`ARCH-006`) can share the same transform.

## Rust module mapping

Consistent with `TOOL-030`:

- `pictura_retouch::clone::CloneSourceSlot` — `{ anchor: Point, transform: Affine2D, offset: (i32, i32), overlay: OverlaySettings }`.
- `pictura_retouch::clone::CloneSourceSet` — fixed 5-element slot array + active index; `select(slot)`, `reset_transform(slot)`, `set_anchor(slot, point)`.
- `pictura_retouch::clone::OverlaySettings` — `{ show, auto_hide, clipped, opacity, mode: OverlayMode, invert }`; `OverlayMode { Normal, Darken, Lighten, Difference }`.
- `pictura_retouch::clone::SampleSurface` — resolves a slot's source surface (shared with Healing Brush).
- `pictura_render::sample::BilinearSampler` / `pictura_transform::Affine2D` — shared mapping types.

Crossing types: `CloneSourceId(u8)`, `Affine2D`, `OverlayMode`, `Point`. No Qt types cross into `pictura_retouch`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `CloneSourcePanel` | `QDockWidget` | Host; five slot buttons, transform/offset fields, overlay controls |
| `CloneSourceModel` | `QAbstractListModel`/`QObject` | Five `CloneSourceSlot` values + active slot; notifies via the Rust bridge |
| `SourceSlotBar` | `QToolButton` group | Source 1–5 selection |
| `TransformFields` | `QDoubleSpinBox` + toggle buttons | W/H/rotate/flip; Maintain Aspect Ratio; Reset Transform |
| `OverlayControls` | `QCheckBox` + slider + combo | Show/Auto Hide/Clipped/Opacity/mode/Invert |
| `SourceOverlayItem` | `QGraphicsItem` (vector overlay) | Draws the transformed source overlay next to the GPU canvas; Move Source Overlay drag |

Widgets over QML for the docked form controls (`ARCH-003`); the overlay is a vector `QGraphicsItem` over the composited canvas, sharing the slot's `Affine2D`, so it never writes pixels itself.

## Data-model impact

- **No PSD fields.** All clone-source state (slots, anchors, transforms, offsets, overlay settings) is **tool session state**, saved only until the document closes. Nothing is serialized (`TOOL-030`).
- **Undo.** Transform/offset/overlay edits are tool state and are not document history. The paint stroke itself is one `PaintStroke` command with pre-edit pixel backups (`ARCH-009`).
- **Pattern Stamp** is a separate tool path that uses a pattern id, not a clone source slot (`TOOL-030`); the panel does not apply to it.
- **Video/animation frame relationship** (when implemented) is timeline state, not clone-panel document data; it must not leak into the PSD as clone state.
- **Preferred source per document** is session-only; whether CS6 restores it per workspace is unverified.

## Edge cases

- **Cross-document sampling** must respect the same color-mode rule as the tool (`TOOL-030`); the panel disables/greys slots whose source becomes invalid.
- **Source document closed** — a slot referencing a closed document must invalidate gracefully, not paint from stale memory.
- **Slot persistence** — exactly "until the document is closed"; a new document starts with empty slots. Closing/reopening a document clears them.
- **Zero/negative scale**, `W`/`H` = 0, or extreme rotation must not divide by zero in the inverse transform.
- **Maintain Aspect Ratio** — editing W updates H (and vice versa) when constrained; independent editing when not.
- **Overlay on a 1-px/empty document** — overlay bounds may be empty; nothing crashes.
- **Clipped + small brush** — overlay clipped to a tiny brush must not produce a zero-size draw error.
- **Opacity 0** — overlay hidden but settings retained.
- **32-bit / CMYK / Lab** — the mode-independent affine mapping applies; the sampler works in the document's channel model (`TOOL-030`).
- **Huge PSB** — overlay must sample tiles lazily; never copy the whole source surface for the overlay.
- **GPU unavailable** — overlay draws on the CPU vector layer; painting still works via the CPU sampler.
- **Aligned/non-aligned interaction** — changing `Aligned` mid-session must not silently corrupt the slot's anchor semantics; the transform/offset remain the panel's, the anchor rule is the tool's.
- **Extended frame controls** — absent in a core build; must be hidden, not shown broken.

## Parity acceptance criteria

1. Given five sampled sources, clicking each source button selects that sampling point without resampling; painting from each uses its slot's pixels.
2. Given the document is closed and a new one opened, all five sources are cleared (session-only persistence).
3. Given `W`/`H` changed with Maintain Aspect Ratio on, the other dimension follows; toggling it off allows independent values; **Reset Transform** restores 100%/100%/0°.
4. Given a rotation value, painted clones match the rotated source within the sampling tolerance; Flip Horizontal/Vertical mirrors the source.
5. Given an Offset x/y, the sampled source is shifted by exactly that pixel offset in the destination.
6. Given `Show Overlay` with Opacity 50%, Invert on, and Clipped off, identical source and destination regions render as solid gray.
7. Given `Clipped` on, the overlay is confined to the brush footprint; given `Auto Hide`, the overlay is hidden during the stroke.
8. Given `Alt+Shift` held with the Clone Stamp, the cursor becomes Move Source Overlay and dragging repositions the overlay without painting.
9. Given `Alt+Shift+<`/`>` or `Alt+Shift+arrow`, the clone source rotates or nudges per the CS6 key table.
10. Given the Clone Source panel used with the Healing Brush, the same slot/transform/overlay state applies.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: `Window > Clone Source` and its purpose for Clone Stamp / Healing Brush; up to **five** sample sources saved **until the document closes**; `Alt`/`Option`-click sampling and selecting a different source button; W/H scale with default proportion constraint and Maintain Aspect Ratio; Rotate entry/scrub; **Reset Transform**; Flip Horizontal/Vertical; Show Overlay and the overlay options (Auto Hide, Clipped, Opacity, Normal/Darken/Lighten/Difference, Invert); the documented "Opacity 50% + Invert, unclipped → solid gray when aligned" registration trick; Move Source Overlay via `Alt+Shift`/`Option+Shift`; Offset x/y; the Clone Source key table (`Alt+Shift` show overlay, `Alt+Shift+arrow` nudge, `Alt+Shift+<`/`>` rotate); the Photoshop-Extended frame-relationship options.
- `https://searxng` query "Photoshop CS6 Clone Source panel five sources offset overlay opacity flip Window menu" — discovery only.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- Adobe's current-version clone/healing help pages surfaced by the query; later-version UI, not asserted as CS6.

Cross-referenced (already sourced in `TOOL-030`):

- `03-tools/clone-stamp-and-pattern-stamp.md` — Clone Stamp sampling model, Aligned/Sample/Ignore Adjustment Layers, source-transform mapping, and the note that the Clone Source panel stores sources until document close.

Not used in this pass:

- `helpx.adobe.com` clone-source pages (HTTP 403 / current-version only).

## Open questions

- **Exact overlay opacity default** (100% assumed) and whether the panel remembers per-slot overlay settings independently. *Resolves with:* a CS6 capture.
- **Overlay blend-mode math** — the four modes presumably reuse standard blend formulas; not confirmed for the overlay (`TOOL-030` shares this question).
- **Sampling interpolation** (bilinear/bicubic/nearest) under scale/rotate is undocumented (`TOOL-030`).
- **Source-slot persistence granularity** — "until the document is closed" is stated; whether a workspace/session restores slots is unverified.
- **Move Source Overlay's exact effect** — does dragging change the stored Offset x/y, the anchor, or both? *Resolves with:* a CS6 test.
- **Extended frame-relationship UI** — exact controls and behavior for video/animation frames are out of core parity (`OVR-003`) and not detailed here.
- **Invalid-slot invalidation rules** when a sampled source document is closed or its color mode changes. *Resolves with:* a CS6 test.
