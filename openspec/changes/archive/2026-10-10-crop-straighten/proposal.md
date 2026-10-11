# Proposal

## Why

CS6's Crop tool rotates the image behind a fixed, axis-aligned crop box (the
"straighten" interaction): the canvas grows to the rotated content's bounding
box so nothing is truncated, and committing crops to the box. Kooka Pictura's
crop is axis-aligned only — it has no angle, tints the whole workspace instead
of the canvas, cannot snap to canvas/layer edges, and cannot rotate during an
active crop because dragging outside the box is bound to drawing a new box.
The engine also lacks a document-scope arbitrary rotation, so the same
straighten capability that Crop needs is the one `Image > Image Rotation >
Arbitrary` needs too.

A UX pass against CS6 (#252) then found the wider tool/panel surface still
short of parity: the crop surface (cursors, box growth, Classic/Modern
semantics, W/H/Resolution fields, the draw-a-line straighten) is wrong in
several places; options bars (Move, Eyedropper) are missing their controls and
no active tool carries a separator; painting refuses a layer nested inside a
group; and the Layers, Channels, Info, and Histogram panels and the footer,
canvas pan, and tool-switch zoom behaviour diverge from CS6. This change is
that audit pass.

## What Changes

- **New straighten-crop interaction**: the crop box stays axis-aligned and
  fixed; the image/layers spin about the box centre; the canvas stays
  axis-aligned and grows to the rotated content's bounding box; commit crops to
  the box.
- **Rotate gesture**: with a box active, any press outside it (beyond the resize
  margin, including past the canvas edge) enters the rotate gesture; a new box
  is drawn only in the init mode where no box exists. Shared with Free
  Transform's outside-corner band.
- **Crop corrections (#252)**:
  - Modern starts with a centered, ratio-fitted **preview** box (dashed, no
    guides, no handles); dragging inside draws a new box, a click adopts the
    preview, and rotating outside activates it. Classic starts boxless. ESC /
    Cancel return to no box in both modes.
  - Dragging a box past the canvas pads the grown area with the background
    colour when a Background layer exists and the transparency checkerboard
    otherwise — the canvas frame always follows the box, not only when
    straightening.
  - Modern drags pan the composite behind a fixed crop box; Classic drags move
    the box.
  - The first box drawn is clamped to the canvas and its press corner stays put;
    a resize afterwards may grow past the canvas.
  - The options bar's `W`/`H` are unit-less ratio/px fields without labels or a
    slider popup and show integer values unless a decimal is typed; the
    `W x H x Resolution` mode shows pixel dimensions and a resolution field
    defaulting to the document's resolution (72 ppi fallback) with a
    non-truncated px/inch · px/cm unit; the `Res:` label is gone.
  - `Cancel` and `Apply` are shown only while a box is active; arming and
    drawing a straighten line from the init/preview state activates the crop and
    previews the rotation immediately.
- **New engine primitive** `rotate_document_in(doc, angle, pivot)`: the
  arbitrary-angle sibling of `rotate_document`, resampling every layer
  (including Background) and expanding the document to the rotated bounding
  box. Reuses the existing oracled `pictura_ops::rotate_in` kernel. Crop
  straighten composes it with the existing `crop_document`.
- **New command** `Image > Image Rotation > Arbitrary`, the primitive's second
  consumer.
- **Modal tool-session undo/redo**: the global Edit Undo/Redo consults the
  active tool before the document history. Crop records `(box, angle)` steps
  and commits exactly one `Crop` state on Apply.
- **Nested-layer targeting (#252)**: the active-layer path resolves to the exact
  leaf layer by walking the tree (`"0/1"`), not as a top-level index; a group is
  not a paint target; ancestors (group mask/clip/blend/opacity/transform)
  contribute only at composite time. Fixes painting a layer nested under a
  folder.
- **Layers panel parity (#252)**: hover hand cursor, single-click lock removal,
  smaller lock badges/header locks, narrower blend select, Background loses its
  lock when masked, mask/vector thumbnails adjacent to the image thumbnail with
  link-2/unlink-2 glyphs, active-thumb brackets and thumb-click activation,
  Alt+click, aspect-correct mask thumbs, folder-row sizing/padding/brackets, a
  responsive visibility toggle, and a chevron on a nested (empty) group.
- **Channels panel parity (#252)**: hover hand cursor; Ctrl shows a
  dashed-square overlay; rows restyled to match the Layers panel.
- **Options bars (#252)**: a separator after every active tool icon; the Move
  bar gains Auto-Select (Group/Layer), Show Transform Controls, and a
  three-dots menu with `Align To:` Selection/Canvas; the Eyedropper gains
  Sample Size, Sample scope, and Show Sampling Ring.
- **Canvas (#252)**: Space / middle-click pan suppresses tool overlays (the
  brush ring); switching tools preserves canvas zoom and position.
- **Info / Histogram / Footer (#252)**: one hint per line, capitalised, 2px
  smaller; the Info icon-menu first open is no longer clipped; Histogram gains
  a default `All Channels` RGB overlay; footer direction buttons use tinted
  Lucide chevrons.
- **Select tools (#252)**: Alt in a marquee subtracts on the first press and
  only mirrors the pivot when Alt is pressed again during the same drag.
- **Not in scope** (own issues): the Transform (Ctrl+T) options bar (#296) and
  Select and Mask / Refine Edge (#297). Content-Aware crop stays a disabled
  placeholder (needs a content-aware fill engine).

## Capabilities

### New Capabilities
- `tools/crop-straighten`: the Photoshop-style rotate-crop interaction — the straighten model, the outside-the-box rotate gesture, the zoned rotate cursor, box growth, snapping, shield scope, the preview/active crop states, and commit semantics.
- `document/layer-nesting`: active-layer path resolution to a leaf, group non-targeting, and composite-time ancestor contribution.

### Modified Capabilities
- `document/image-orientation`: add document-scope arbitrary rotation `rotate_document_in(doc, angle, pivot)` and the `Image > Image Rotation > Arbitrary` entry point.
- `tools/tool-framework`: modal tool-session undo/redo; the Crop options-bar page (ratio-vs-resolution fields, the `Ratio` entry, the Straighten line toggle, separators, Classic/Modern); a separator after every active tool icon; the Move options-bar additions; the Eyedropper options bars.
- `tools/canvas-tools`: the Crop tool requirement changes — preview/active states, shield scope, snapping, box growth, the rotate-when-active rule, and Modern panned repositioning.
- `ui/svg-cursors`: the shared tool-arrow is filled black beneath its white outline in every tool cursor built on it.
- `ui/layers-panel`: the panel parity items above.
- `ui/channels-panel`: hover cursor, Ctrl overlay, Layers-style rows.
- `ui/info-histogram-panel`: Info hint rendering and menu fix; Histogram All Channels.
- `ui/document-canvas`: Space/middle pan overlay suppression and tool-switch zoom/position preservation.
- `tools/tool-hint-bar`: Lucide chevron direction buttons.
- `tools/selection-tools`: the two-stage Alt marquee behaviour.

## Impact

- Engine: `crates/pictura-render` (new `rotate_document_in`; grow instead of clamp in `crop_document`; leaf-path resolution for active-layer targeting), `crates/pictura-ops` (reused `rotate_in`), `crates/pictura-render/tests/document_oracle.rs`.
- App Rust bridge: `crates/pictura-app/src/cxxqt_object*` (crop/rotate entry, active-layer resolution, eyedropper sampling size/scope, Move/Auto-Select/align-to state, Layers active-thumb/lock).
- App C++: `tool_crop.cpp`, `crop_grip.h`, `image_view.{h,cpp}` + `image_view_overlays.cpp`, `options_bar*.cpp/.h`, `panels/numeric_field.*`, `layers_panel*.{cpp,h}`, `channels_panel.cpp`, `panels/info_panel.cpp`, `panels/histogram_panel.{h,cpp}`, `tool_hint_bar.cpp`, `tools.{h,cpp}`, `tool_context.h`, `tool_handler.h`, `tool_selection.cpp`, `selection_geometry.{h,cpp}`, `tools_marquee.cpp`, `tool_eyedropper.cpp`, `tool_move.cpp`, `frame*.cpp`, `icons.{h,cpp}`, `tool_transform.cpp`.
- Assets: `assets/cursors/*.svg`, `assets/icons/*.svg` (link-2/unlink-2, chevron-up/left), `lucide-map.json`, `pictura.qrc`.
- Tests: Rust oracle/unit tests for `rotate_document_in`, padded crop, and nested-layer paint; Qt Tests for the crop session/zones/options/growth/preview, layers/channels/info/histogram panels, options bars, selection Alt, canvas pan, and footer.
- No new dependency.
