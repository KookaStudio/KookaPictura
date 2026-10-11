# Design

## Context

See `proposal.md` — Why. Current state that shapes the approach:

- `CropToolHandler` (`cpp/tool_crop.cpp`) holds an axis-aligned `QRectF box_` in image space; `cpp/crop_grip.h` owns handle hit-testing, drag, ratio fit, and cursors. There is no angle in the model.
- The commit bridge is `crop_to(...)` → `pictura_render::crop_document` (axis-aligned) + `delete_cropped_pixels`, then `record("Crop")` (`cxxqt_object/crop_group.rs`).
- Crop overlays are painted in widget space (`image_view_overlays.cpp`) but coordinates go through `toWidget`, which folds in `ImageView::viewRotation()`; the shield uses `viewRect()`, dimming the whole viewport.
- `ImageView` already rotates its paint (`rotation_`/`viewRotation`, applied at `image_view.cpp:719`/`:776`) for the Rotate View tool, and clips to the unrotated `docDevice` (`:779`).
- The engine has exact quarter-turn `rotate_document` and per-layer `transform_layer`, which **refuses Background**, plus the oracled buffer kernel `pictura_ops::rotate_in`.
- The Edit Undo/Redo handlers call `view->undo()/redo()` directly (`frame_menus.cpp:196`).

## Goals / Non-Goals

**Goals:**
- A durable document-scope arbitrary rotation primitive with a second, non-crop consumer.
- Crop straighten that keeps the box and canvas frame axis-aligned.
- One shared rotate-gesture convention across Free Transform and Crop.
- A reusable modal tool-session undo mechanism, not a crop one-off.
- Crop surface parity with CS6: zoned rotate cursor, ratio-vs-resolution fields,
  the `Ratio` entry, the Straighten line gesture, and box growth.

**Non-Goals:**
- Content-Aware crop (needs a content-aware fill engine); the options-bar
  `Content-Aware` checkbox ships as a disabled placeholder.
- Perspective crop (already exists) and view rotation (already exists).
- Reinterpreting `delete_cropped_pixels`: growing the canvas only pads, it never
  changes how off-canvas pixels are trimmed.
- The Transform (Ctrl+T) options bar (#296) and Select and Mask / Refine Edge
  (#297): separate features with their own issues; this change only removes the
  crop/panel/tool-bar gaps around them.
- Resolution-driven resampling on crop commit: the crop engine isolates pixels;
  the Resolution field is a print-size/metadata hint in this change (see D18).

## Decisions

### D1 — Straighten model: fixed box, spinning content, box-sized output
The crop box stays axis-aligned and fixed; content rotates about the box centre; the canvas stays axis-aligned and grows to the rotated content's bounding box. Commit yields the box-sized result. Alternative — tilt the box and commit the tilted rectangle — was rejected because it does not match CS6 and makes the output dimensions angle-dependent and unintuitive.

### D2 — Engine: document-scope `rotate_document_in`, reused by crop and by `Image > Arbitrary`
Add `pictura_render::rotate_document_in(doc, angle, pivot)` as the arbitrary-angle sibling of `rotate_document`. It resamples every layer's channels and mask through `pictura_ops::rotate_in`, expands document bounds to the rotated bounding box, and remaps every rect/bound/channel.

- Chosen over **per-layer `transform_layer` composition**: that op refuses Background and position-locked layers, so it cannot carry the common opened-photo case, and it would need a separate Background path — the drift risk the repo warns about.
- Chosen over a **monolithic `straighten_crop`**: keeping rotate and crop as two composable, separately oracle-testable ops mirrors the existing `rotate_document`/`crop_document` pair and lets `Image > Arbitrary` reuse rotate alone.
- Sample kernel is reused, so sampling correctness inherits the existing oracle.

### D3 — Preview: content rotation in `ImageView`, screen-aligned overlays
Add a content-rotation preview (angle + pivot) that rotates the drawn image and expands the clip to the rotated bounding box instead of `docDevice`. Crop overlays draw through an axis-aligned (screen) path so box and handles never tilt.

- Alternative — reuse `viewRotation` wholesale: rejected because `toWidget` folds the view rotation into overlay coordinates, which would tilt the crop box.
- The preview builds from the view-pyramid level, not full resolution, and caches per `(angle, docRect)`, consistent with the existing present cache.

### D4 — Gesture: rotate whenever a box is active
Crop enters the rotate gesture on any press outside the box, beyond a resize margin and including past the canvas edge; the new-box draw is reserved for the init mode where no box exists (matching CS6 and #252). This supersedes the original outside-corner band for Crop; both Crop and Free Transform share the margin helper so the resize handles never fight the rotate zone.

### D5 — Sub-history: generic modal tool session
Add `toolUndo()`/`toolRedo()` (default false) to `ToolHandler`; the global Edit Undo/Redo handlers consult the active tool first, then fall back to the document history. The Crop session holds `(box, angle)` snapshots pushed on drag release, discarded on Cancel/deactivate, and collapsed by the single `record("Crop")` on Apply. Chosen over a crop-local stack because Free Transform and text are the same shape and would otherwise each reinvent it.

### D6 — Pivot is the crop-box centre
Matches CS6; keeps the box visually stationary while content spins. Alternatives (canvas centre, layer centre) move the box on screen.

### D7 — `Image > Image Rotation > Arbitrary` wired now
Makes `rotate_document_in` a two-consumer contract from day one, forcing a general API (angle, pivot, interpolation, background) instead of a crop-shaped one.

### D8 — Grow the canvas to the box
`crop_document` gains a non-clamping mode: when the committed box extends past
the canvas, the document grows to the box and the new area is padded — the
background colour when the document has a Background layer, transparent
otherwise — reusing `extend_channel`, which already zero-pads and accepts a
negative origin. The clamp path stays for `Image > Crop` and selections. The
first box a user draws is clamped to the canvas in the handler, so the canvas
only grows through a deliberate handle resize afterwards.

### D9 — Zoned rotate cursor (eight orientations)
Lift Free Transform's heading→rotated `cursor.rotate` helper into
`icons.{h,cpp}`. Crop quantizes the pointer's heading around the box centre to
the nearest of eight zones — four corner diagonals and four edge axes — and
widens the corner wedges (about ±30° instead of ±22.5°) so the corner variant is
not a thin sliver. Continuous heading was rejected because #252 asks for the
eight discrete variants.

### D10 — Straighten is a line gesture, not a field
Replace the `Straighten` numeric field with a toggle that arms a measure mode:
press, drag, release draws a line; the release sets the crop angle to the line's
inclination so the drawn horizon becomes horizontal, then disarms. The line is
previewed in `ImageView`. Chosen over keeping a numeric field because CS6's
control is the line gesture.

### D11 — Classic and Modern both move the box
Both modes drag the crop box; the content-pan offset is removed. The mode now
only decides the initial box (Classic boxless, Modern full-canvas) and is kept
for the persisted setting and the cog menu. Matches #252's "only the crop box
should be draggable".

## Decisions — UX audit (#252)

### D12 — Crop states: none → preview → active
The handler gains a `preview` state distinct from `active`. Modern's initial box
is a **preview**: dashed outline, no rule-of-thirds guides, no handles, and no
Cancel/Apply. A drag inside the preview draws a new box; a press+release without
movement adopts the preview as active; a rotate press outside activates it. A
box becomes active on any of those transitions. Classic's initial state is
`none` (boxless). ESC / Cancel return to `none` in both modes. The render mode
lives in `ImageView` (a `setCropPreview` flag) so overlays can draw dashed-only.

### D13 — Modern starts centered; both modes drag; Modern repositions the composite
The Modern initial box is fitted to the ratio **and centered** on the canvas (not
anchored at the corner). Classic drags move the box; Modern drags pan the
composite under a box fixed in workspace coordinates, via a session-scoped
content offset in `ImageView` (the box overlay is drawn without it, so it stays
put) and the commit maps `box − offset`. This restores the pre-10.5 Modern
behaviour that #252's tutorial step "click within the image and drag to
reposition the image within the crop box" describes.

### D14 — The canvas frame follows the box at every angle
The paint path uses `cropCanvasImageRect()` (image united with the box, and the
rotated bounding box under straighten) whenever a box exists, not only when
straightening. The grown area is filled with the canvas backdrop (background
colour when layer 0 is a Background, else the checkerboard), matching the
shield, which already used the unioned rect.

### D15 — Draw-new pins the press corner
`onPress` records the press point; ratio fitting for a fresh draw anchors on that
point for any drag direction, so the start corner never drifts (the old code
hard-coded the `BottomRight` grip, which moved the press corner when dragging
up/left).

### D16 — W/H/Resolution fields
`W`/`H` lose their leading labels and the slider popup; the validator accepts
decimals and the formatter drops trailing zeros unless a decimal was typed. In
`W x H x Resolution` mode they show pixel dimensions and a resolution field (no
`Res:` label) whose unit combo (`px/in` · `px/cm`) sizes to its contents. The
resolution defaults to `document_ppi(view)` (the ResolutionInfo resource, CS6's
72 ppi fallback) — see D18.

### D17 — Cancel/Apply visibility keyed on crop state
The options bar shows Cancel/Apply (and Reset) only while a box is active; the
page re-syncs from a crop-state signal (`toolSessionChanged` plus
`cropOptionsChanged`) and a `cropActive` query, not just on tool switch.

### D18 — Resolution is metadata-only for now
CS6 resamples to `(W,H) × Resolution` when a physical unit and a resolution are
set. Our crop engine isolates pixels, so this change sets/reads the field and
the document resolution but does not resample on commit; the print size is
recomputed instead. Marked with a `ponytail:` ceiling; resampling is a future
engine step.

### D19 — Straighten activates from the init/preview state
A straighten-line press in `none`/`preview` first activates a box (Classic: full
canvas; Modern: the preview box), so the pivot exists and the rotation preview
updates from the first drag frame and instantly on release.

### D20 — Nested layers: resolve to the leaf, ancestors at composite time
The active-layer path is resolved by walking the layer tree (`"0/1"` = child 1 of
node 0) via the existing `pictura_render::resolve_path(_mut)`. A resolved group
is not a paint/fill/filter target (matching Photoshop). The leaf is edited in
isolation; ancestor group mask/clip/blend/opacity/transform apply only when the
tree is collapsed (Pass Through = no isolation; Normal = merge then blend).
Secondary top-level `usize` parsers in the app bridge and the two C++ top-level
`toInt` lock/kind queries are converted to path-based lookups.

### D21 — Options-bar separator is generic
One shared `separator(page)` + `layout->insertWidget(1, line)` in
`OptionsBar::buildPage` after `toolButton(...)` covers every page (including the
`default:` branch), instead of each builder adding its own.

### D22 — Panels reuse existing surfaces and delegates
The Layers items are localized to `LayerRowDelegate`/`LayerRow` (thumb geometry,
lock badge, brackets, folder sizing) plus a few `LayersPanel` event branches;
Channels gets a small `QStyledItemDelegate` and `Theme::shade` surfaces to match
Layers. No new abstraction is introduced where a delegate method suffices.

### D23 — Canvas pan/zoom fixes are local
`resizeEvent` stops re-fitting after the first post-load sizing (so switching
tools cannot snap zoom); `setSpacePan`/middle-press repaint and the brush ring is
gated on `!panning_ && !spacePan_`.

## Risks / Trade-offs

- **Preview cost at large documents** → build the preview from the pyramid level and cache per angle; do not resample full-res per frame.
- **Special-store remap after rotation** (type, smart object, vector mask, 16/32-bit) → remap in `rotate_document_in` under the same rules `crop_document`/`delete_cropped_pixels` already use; cover with the document oracle round-trip and a kind-matrix scenario.
- **Undo routing regressions** (tool session masking document undo) → default hooks return false; explicit fallback scenarios and a Qt Test for the session.
- **Oracle tolerance for arbitrary angles** → reuse the tolerance already measured and recorded for `rotate_in` in `pictura-ops/tests/oracle.rs`; right angles stay bit-exact.
- **Angle 0 must stay a no-op** → `rotate_document_in` early-returns unchanged, so choosing Crop adds no history and no reinterpretation.
- **Growing past the canvas with a Background layer** → pad the Background channel with its colour and other layers transparent, matching the backdrop rule; cover with a Rust unit test. An inside-canvas crop keeps clamping bit-for-bit.
- **Corner rotate zones too thin** → widen the corner wedges and cover the eight zones with a Qt Test.

## Migration Plan
No data migration: `rotate_document_in` and the tool session are additive; an
axis-aligned crop whose box stays inside the canvas is unchanged. Rollback is
reverting the change; no on-disk format change.

## Open Questions

- Interpolation for `rotate_document_in`: bilinear only (matching `rotate_in`) for now, or expose a kernel choice for `Image > Arbitrary`? Bilinear-only is sufficient for the specs; a kernel parameter can be added without a spec change.
