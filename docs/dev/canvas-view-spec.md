# Canvas & View Behaviour Spec

- **Status:** dev note (not a milestone proposal)
- **Scope:** how the document canvas/view should behave, the Move-tool drag
  contract, and the interactive performance budget. Written against the
  implementation at `crates/pictura-app` (ImageView, ToolController, frame) and
  `PictureView` (cxxqt_object.rs).
- **Companion specs:** `03-tools/hand-and-zoom.md` (TOOL-042),
  `03-tools/move-and-transform.md` (TOOL-001),
  `02-ui-ux/panels/navigator-panel.md` (PAN-014),
  `02-ui-ux/application-frame.md` (UI-001),
  `01-architecture/performance-targets.md` (ARCH-013),
  `01-architecture/gpu-rendering-pipeline.md` (ARCH-006).

Facts below are marked **CS6** (stated in the archived CS6 Help PDF or a
first-party Adobe page), **CS6-sec** (secondary/community but consistent), or
**[ext]** (a Kooka Pictura extension that CS6 does *not* do by default).

---

## 1. Intended canvas behaviour

### 1.1 Initial view on open/new

- **CS6-sec:** an image opens **Fit-to-Screen** when it is larger than the
  window, and centred; a document that already fits opens at 100%. Adobe
  support and current-product docs describe Fit-on-Screen as the default open
  view, and users ask how to *disable* it, which confirms it is the default.
  The CS6 Help PDF documents `View > Fit On Screen` and `100%` but does **not**
  print the open default, so the "fit when larger" rule is secondary-confirmed,
  not PDF-confirmed.
- **CS6:** zoom and scroll are **view state**, not document content; they are
  not history states and must not appear in the History panel.
- **Current code:** `ImageView::setImage` resets `zoom_ = 1.0` and
  `offset_ = QPointF()` (`image_view.cpp:24`). `(0,0)` is the **top-left**, not
  centre, so a document opens pinned top-left at 100%. `Frame::addDocument`
  calls `setImage` (`frame.cpp:251`), and `fitOnScreen`/`actualPixels` are never
  called on open.
- **Required:** after a document is loaded, set the view to fit-and-centre when
  the document exceeds the viewport, else 100% centred. Reuse
  `ImageView::fitOnScreen` (it already computes `min(w/doc_w, h/doc_h) * 0.95`
  and centres) rather than reimplementing the math. `setImage` should centre
  even in the 100% case (its own doc comment already claims "centred"; the code
  contradicts it).
- **Scroll bars / overscroll** are part of CS6 Standard Screen Mode (UI-001);
  out of scope for this note beyond "the view must be centre-able and pannable".

### 1.2 Zoom

| Action | Gesture | Status |
|---|---|---|
| Fit on screen | `Ctrl+0`, double-click Hand, options-bar Fit | **CS6** |
| 100% (Actual Pixels) | `Ctrl+1`, `View > Actual Pixels` (CS6 label) | **CS6** |
| Zoom In / Zoom Out | `Ctrl++` / `Ctrl+-`, next preset step | **CS6** |
| Zoom tool click | next preset step, centred on click point | **CS6** |
| Wheel zoom | `Zoom With Scroll Wheel` preference (default off in Photoshop) | **CS6**; this app's wheel zoom is always-on, i.e. a deviation to note |
| Max / min | 3200% max, 1-pixel effective min; Zoom In/Out disable at bounds | **CS6** |

- **CS6:** the zoom ladder is a **preset sequence**, not a fixed multiplier.
  Only 25 / 33.3 / 50 / 66.7 / 100 are sourced; the full ladder
  (1, 2, 3, 4, 6, 8, 12, 16, 25, 33.3, 50, 66.7, 100, 200, 300, 400, 600, 800,
  1200, 1600, 2400, 3200 %) is inferred. Exact CS6 set remains unverified
  (open question in TOOL-042).
- **Current code:** `zoomIn`/`zoomOut` step by a fixed ×1.2/÷1.2, and the wheel
  uses `pow(1.0015, angleDelta)` (`image_view.cpp:38-58`). Neither follows a
  preset ladder. The spec requirement is the ladder; the exact fraction is a
  follow-up decision (recommend matching the CS6-observed 25/33.3/50/66.7/100).
- Zoom must not resample or reallocate the document; it is a view transform
  (`ARCH-006` says the transform belongs to the view). Current paint already
  does this: `paintEvent` scales the `QPainter` (`image_view.cpp:124-126`).

### 1.3 Pan

- **CS6:** Hand tool + drag pans. Holding `Space` with another tool temporarily
  gives the Hand tool. Flick panning (OpenGL) is a release-momentum extension.
- **CS6:** the Navigator panel's proxy box pans; the scroll bars pan.
- **Current code:** panning is wired only when `panEnabled_` is true, and
  `setPanEnabled(active_ == ToolId::Hand)` (`tools.cpp:194`, `image_view.cpp:143`).
  So panning works with the Hand tool but **not** with Space (no key handling
  exists), and middle-drag is not handled at all.
- **Required:** middle-button drag pans the canvas **regardless of the active
  tool**; the cursor should reflect a pan (open/closed hand). Middle-drag with
  the left button also held may switch to the rotate/zoom-box form as CS6 does,
  but that is optional here.
- **[ext] Middle-mouse pan is a Kooka Pictura extension.** CS6 does **not** pan
  on middle-drag by default; community threads and third-party AutoHotkey
  scripts exist specifically to add it. Implement it deliberately as an
  extension, documented as such.
- **CS6-sec:** Space+drag is CS6 behaviour and should be added; it is currently
  missing. It differs from middle-drag (which is the extension) and should be
  implemented too.
- **[ext] Workspace snap (`View > Snap Canvas To Workspace`).** A checkable
  option, off by default, that pins the canvas to the workspace centre while the
  scaled document fits inside the viewport: the offset is forced to the centre,
  panning is refused, and the workspace scroll bars hide. Once the scaled
  document overflows either axis the option has no effect and the free-pan
  policy governs, with the scroll bars shown again. It is independent of
  `View > Free Pan` (#414), which is per-axis and controls whether an axis that
  fits may be panned off-centre at all.
- Wheel zoom-in stays cursor-anchored in both modes: while the workspace snap
  lock holds, the fitted canvas is re-centred after each zoom so the whole
  document stays centred; once the document overflows, the point under the
  cursor stays fixed exactly as in free mode.

### 1.4 Status-bar zoom readout

- **CS6:** the status bar is at the bottom of every document window and shows the
  current **magnification** and file size; its triangle opens the view-option
  menu (Document Sizes, Document Profile, Document Dimensions, Scratch Sizes,
  Efficiency, Timing, Current Tool, …). The zoom field is editable.
- **Current code:** `zoomLabel_` reads `canvas->zoom() * 100`
  (`frame.cpp:1322-1343`), updated on `zoomChanged` and on frame refresh. This is
  the right source of truth. Keep all zoom surfaces (status bar, Navigator,
  View menu, Zoom tool) reading/writing the one `ImageView` zoom.

### 1.5 Navigator relationship

- **CS6:** the Navigator thumbnail shows the whole artwork; the proxy/view box
  maps the viewport. Drag the box (or click the thumbnail) to pan, drag the
  slider / type a value / use the mountain buttons to zoom, `Ctrl`/`Cmd`-drag
  to set size+position at once. Panel zoom and the status-bar zoom are the same
  value.
- **Current code:** `NavigatorPanel` owns a slider and Fit / 100% buttons, fed
  by `syncFromCanvas()` and `ImageView::zoomChanged` (`navigator_panel.cpp`).
  It is a separate `QWidget` custom paint, not the GPU pyramid CS6 uses.
- **Required:** Navigator stays in sync with the canvas transform, and its
  repaint must be **coalesced/throttled** so it does not repaint the full
  composite on every `changed` (see §4). The thumbnail should derive from the
  existing display image, not trigger a new full composite.

### 1.6 Behaviour summary

| Behaviour | Source | Current code |
|---|---|---|
| Open fit-and-centre when larger than window | CS6-sec | missing (opens top-left, 100%) |
| 100% centred on `Ctrl+1` | CS6 | `actualPixels` centres, but not bound to shortcut |
| Fit on `Ctrl+0` | CS6 | `fitOnScreen` exists, binding status to verify |
| Preset zoom ladder | CS6 (exact set inferred) | fixed ×1.2 |
| Wheel zoom about cursor | CS6 pref; always-on here | present (`zoomAt`) |
| Hand-tool drag pan | CS6 | present |
| Space+drag pan | CS6 | missing |
| Middle-drag pan | **[ext]** | missing |
| Workspace snap: pin a fitting canvas to centre, hide the bars | **[ext]** | present |
| Navigator pan/zoom, synced | CS6 | present, but unthrottled |
| Status-bar zoom readout | CS6 | present |

---

## 2. Move tool behaviour

- **CS6:** with the Move tool, dragging "drags any object onto one of the
  selected layers; all objects on the layer move together." The layer content
  visibly follows the pointer during the drag; the pointer does not wait for the
  button to be released. Arrow keys nudge 1 px, `Shift`+arrow 10 px.
- **CS6-sec:** a series of nudges counts as **one** history step; by extension a
  single drag is one committed move. One history state per commit.
- **CS6:** `Show Transform Controls` (formerly *Show Bounding Box*) draws a
  bounding box + handles on the active layer/selection; the handles drive
  scale/rotate without entering Free Transform. `View > Show > Layer Edges`
  outlines the selected layer's content.
- **Current code:** `ToolController::handleMoved` for `Move` only accumulates
  `totalDelta_` and returns (`tools.cpp:301-305`); the document is not touched
  until `handleReleased`, which calls `PictureView::translate_layer`
  (`tools.cpp:350-357`). So there is no live preview: the layer jumps on
  mouse-up. That is the reported 1–2 s delay (the drag duration plus the commit
  cost).
- **Required:**
  - Preview the moved layer **live** during the drag. Cheapest correct form:
    draw the composited image with an offset translation for the active layer
    (or a simple visual offset of the whole composite while the drag is in
    progress), rather than recompositing per mouse-move. A true per-layer live
    composite is the eventual form (`ARCH-006` dirty-rect recompose), but the
    minimum acceptable preview is "the pixels follow the cursor at frame rate".
  - Show feedback while dragging: the moved layer's bounding-box outline (or
    the `Show Transform Controls` box + handles when the option is on).
  - Commit **once**, on mouse-up, with exactly **one** history state ("Move
    Layer"). Nudges with the arrow keys should also collapse into a single
    history state per burst.
  - The live preview is transient and **not** undoable; `Ctrl+Z` after a move
    reverts the whole move in one step.
  - Out of scope here: Auto-Select, Align/Distribute, Free Transform handles,
    `Alt`-drag duplicate. They belong to TOOL-001.

---

## 3. Performance budget

### 3.1 Existing targets (ARCH-013)

| Workload | Metric | Budget | Basis |
|---|---|---|---|
| Pan (drag) | Frame rate | 60 FPS sustained, 24 MP doc | Target |
| Zoom (scrubby/continuous) | Frame rate | 60 FPS sustained, 24 MP doc | Target |
| Rotate view | Frame rate | 60 FPS, 24 MP doc | Target |
| Brush stroke | Input-to-first-pixel | ≤ 16 ms | Target |
| Brush stroke | Sustained updates | 120 Hz sampling, ≥ 60 FPS redraw | Target |
| Open document, 100 MB PSD | Time | ≤ 2 s | Target |
| Cold start | Time to interactive | ≤ 3 s | Target |

Frame decomposition for pan/zoom: 16.6 ms total; input ≤ 1 ms, core tile gather
≤ 3 ms, GPU composite + present ≤ 8 ms, Qt scene-graph ≤ 2 ms, slack ≤ 2 ms. No
more than 10 ms may be spent in the Rust core and GPU submit path. CPU fallback
is allowed ≤ 2× the GPU target. Acceptance measures **p95 frame time ≤ 20 ms**
over a 5-second gesture.

### 3.2 Proposed interactive targets for this work

These extend ARCH-013 to the canvas and Move tool. All are **Targets** to be
validated on the reference machine.

| Operation | Metric | Proposed budget |
|---|---|---|
| Pan / zoom / hand + middle drag | Present a frame within | 16.6 ms (p95 ≤ 20 ms) |
| Any view gesture on GUI thread | Single input handler blocks GUI thread for | ≤ 8 ms |
| Move drag preview | Frame while dragging | 16.6 ms; no full-document recomposite per mouse-move |
| Move drag event handling | GUI-thread work per `mouseMoveEvent` | ≤ 2 ms (accumulate + repaint request only) |
| Move commit (mouse-up) | GUI-thread time, 24 MP doc | ≤ 100 ms, once; may be O(document) |
| Open / document op / global filter | — | may be O(canvas), bounded by ARCH-013 |
| Brush dab | GUI-thread work per dab | ≤ 16 ms (already a Target) |

### 3.3 What may be O(canvas) vs must be incremental

**May be O(canvas)** (one-shot, not per frame):
- open, save, import;
- document-wide ops (crop, resize, rotate, flip, canvas resize, mode switch);
- global filters and adjustments;
- one **move/transform commit** (building the new composite once) and its
  history snapshot.

**Must be incremental / O(viewport) or O(changed region):**
- pan and zoom (transform only; must not touch document pixels) — **met**: the
  canvas crops a `pictura-render::ViewPyramid` level chosen for the zoom
  (`ImageView::presentLevelForZoom` + `PictureView::display_image`), so a pan or
  hover repaint never rescales the full-resolution document, and a live paint
  stroke presents the same crop (the regional refresh folds every dab into the
  pyramid, so `presentCrop` no longer bails while painting). Level 0 is the one
  cached sRGB frame that the pyramid borrows, levels below are stored
  premultiplied, and the display `QImage` is built from level 0 on demand rather
  than cached, so only one full-resolution frame is held; this is CPU
  display-only and does not composite layer proxies (the deferred GPU-tile track
  below);
- Move-tool drag preview (translate a cached layer/composite; no recomposite per
  event);
- brush dabs (only the dab's bbox, not the whole canvas);
- layer thumbnails (only the layer that changed);
- histogram and Navigator thumbnail (the histogram bins a view-pyramid level, so
  its cost is independent of document size and it no longer builds the full
  image);
- undo/redo of a move (restore the snapshot, no re-composite storm).

### 3.4 Measured brush-dab split (reference machine, release build)

`stroke_split_profile_4000` (`cargo test --release -p pictura_app
stroke_split_profile -- --ignored --nocapture`) splits a stroke's per-event cost
into the document clone, stroke start, one dab's rasterization, four further
dabs, and the region present. 4000×4000 RGB/8, hardness 100, spacing 25 %, GPU
region composite:

| Layers | Diameter | `Document::clone` | Stroke start | First dab | Four more dabs | Region present |
|---|---|---|---|---|---|---|
| 1 | 64 | 0.004 ms | 5.3 ms | 21.8 ms | 3.3 ms | 0.40 ms |
| 1 | 500 | 0.004 ms | 5.5 ms | 24.1 ms | 20.2 ms | 1.20 ms |
| 8 | 64 | 0.008 ms | 0.4 ms | 21.5 ms | 3.3 ms | 0.55 ms |
| 8 | 500 | 0.008 ms | 5.5 ms | 28.0 ms | 20.0 ms | 4.22 ms |

Two changes moved these numbers, and the table carries both:

- **Rasterization** dropped from 18.5 to about 10 ms per full-disc dab by
  hoisting the stroke-constant tip profile and the layer's plane positions out
  of the pixel loop; the first-dab column still reads higher (21–28 ms) because
  the first write also pays the one-time copy-on-write fork of the painted
  layer's four planes (64 MB at 4000²), which the old code paid up front as two
  full-document clones instead.
- **`Document::clone` is a refcount bump** (0.004–0.008 ms) rather than a copy,
  so stroke start fell from 81 ms with one layer and 360 ms with eight to under
  6 ms, and a history state now shares its pixels with the document until one of
  them is written.

Brush sizes run 1–5000 px (`options_bar_paint.cpp`), so the 500 px column is the
midpoint of the range. `large_brush_profile_4000` sweeps the upper half of that
range on the same document, with the one-time plane fork reported separately
because it is a fixed cost the brush does not change:

| Diameter | Stroke start | First dab | Region present |
|---|---|---|---|
| 500 | 5.3 ms | 34.7 ms | 1.2 ms |
| 1000 | 5.4 ms | 59.9 ms | 5.4 ms |
| 2000 | 0.9 ms | 201 ms | 24.6 ms |
| 5000 | 1.1 ms | **933 ms** | **133 ms** |

A dab's cost is O(its bounding box), and above the 262 144 px budget (⌀≈512) the
reduced-resolution stroke preview takes over: while the button is down the app
rasterizes the accumulated dabs' coverage at view-pyramid level 3 (1/8) over a
snapshot of that level, patches it in place, and forces the canvas to present it
until the stroke ends. `end_paint` replays the logged samples through a real
full-resolution `Stroke` before recording history, so the committed pixels are
byte-identical to a stroke that never previewed; `cancel_paint` drops the preview
and restores. The exact stroke is still O(area) at release, which is the recorded
ceiling; self-test check 545 proves a ⌀512 stroke on a 2048² document presents
level 3 live and the post-commit canvas matches a full recomposite byte-for-byte.

When a Vulkan adapter is present and "Use Graphics Processor" is on, the
**GPU stroke** replaces that preview for a Normal or Clear stroke whose dab is
over the same raster budget (a smaller dab is a few hundred microseconds on the
CPU, less than the ~65 ms it costs to seed the GPU with a full-frame upload at
4000²): the target layer is uploaded once to a resident GPU buffer (plus an
immutable base copy and a per-pixel coverage plane), each dab is one compute
dispatch over its bounding box (the same tip curve the CPU evaluates) that
recomposites every changed pixel from the base and writes the target layer, and
only the dab's changed rectangle is read back and patched into the exact
`Stroke`'s working document. The present composites that document through
`refresh_region` and stays frame-bounded, and `end_paint` commits the
GPU-authored document with no replay, so the committed pixels equal the GPU
stroke and lie within ±1 LSB of a CPU-only stroke (self-test 546 `pp_gpu_commit`
and `gpu_stroke_matches_the_cpu_oracle`). Otherwise the preview (over budget) or
the exact CPU path runs unchanged.
---

## 4. Hot spots — findings and resolution

### Paint-path timing (opt-in)

Set `PICTURA_PAINT_TIMING=1` and paint; the engine logs a per-phase summary at
each commit and the Qt frontend logs each phase to stderr as it happens:

```
PICTURA_PAINT_TIMING=1 ./build/pictura 2>/tmp/paint.log
```

Rust phases (grouped under `[paint-timing] ===== stroke (begin -> commit) ====`):
`begin_total`, `stroke_begin_at (doc clone)`, `gpu_seed_interleave`,
`gpu_new (alloc+upload)`, `preview_new (snapshot)`, `dab_total`,
`gpu_dab_dispatch_readback`, `gpu_patch_working_layer`, `cpu_stroke_sample`,
`cpu_present_region`, `commit_replay_cpu_stroke` (preview strokes only),
`commit_finish`, `commit_refresh_region`,
`commit_record_history`, `end_paint_total`, and the `refresh_region` internals
(`rr_*`). Qt phases (`cxx_*`): `begin_paint (call)`, `paint_dab (call)`,
`end_paint (call)`, `frame_refresh (total)`, `view_image()`, `replaceImage`,
`blitRegion` / `blit_detach_image`, `regionBlitted_slot`, `paintEvent`,
`retargetDock (all panels)` and each panel's refresh. The switch is a no-op when
unset.

Findings from reading the code. Items 1 and 4 (the Move-tool per-event
composite) are **resolved** in the post-M24 pass; items 2, 3, 5, and 6 remain
open.

1. **Double full composite per commit, GUI thread — RESOLVED (Move tool).**
   `PictureView::translate_layer` calls `pictura_render::translate_layer`
   (`crates/pictura-render/src/document_ops/crop.rs:51`), which itself calls
   `recompute` (`composite_rgba`) — then `PictureView` calls `recomposite`
   (`cxxqt_object.rs:931-944`), whose `document_to_image` → `current_buffer`
   calls `composite_rgba` **again** because `doc.layers` is non-empty
   (`cxxqt_object.rs:1829-1840`). Then `buffer_to_image` does a per-pixel
   planar→RGBA loop (`cxxqt_object.rs:1904-1937`) and allocates a fresh
   `QImage`. Two full composites + a full conversion per move/recomposite, on
   the GUI thread. Confirm with a profiler before optimising; the fix direction
   is to composite once with a dirty-rect/incremental path and cache the packed
   image.

   **Measured (post-M24 pass):** a single 1024×1024 two-layer `composite_rgba`
   costs ~**257 ms in the debug build** (the old CMake default) and ~**39 ms in
   the optimized build** (~6.6×). Each Move mouse-move ran `move_preview` →
   `pictura_render::translate_layer` (full composite) then `document_to_image`
   (a second full composite) plus a full planar→RGBA conversion, so two
   composites per pointer event made dragging unusable.

   **Fix 1 — preview in Qt, zero compositing during the drag.** The bridge
   caches a preview: `begin_move_preview()` stores a base image (the document
   composited with the moved topmost raster layer hidden), the layer's own
   image, its document-space top-left, and its opacity; `move_preview_base()`,
   `move_preview_layer()`, `move_preview_x()`, `move_preview_y()`,
   `move_preview_opacity()`, and `end_move_preview()` expose/clear it.
   `commit_move(dx,dy)` applies the real move once (a single composite) and
   records exactly one "Move Layer" state. The old per-event
   `move_preview(dx,dy)` is retained only for the self-test (`// ponytail: slow
   path`). `ImageView::beginMovePreview(base, layer, layerPos, opacity)` +
   `setMovePreviewDelta(delta)` + `endMovePreview()` draw the cached base then
   the moved layer at the live delta (source-over with `setOpacity`) in
   `paintEvent`, so a drag composites nothing. `ToolController` Move press calls
   `begin_move_preview` and seeds the canvas preview; move only updates the Qt
   delta; release calls `end_move_preview`, then `commit_move(dx,dy)` once when
   the delta is non-zero, then `endMovePreview`. Switching tools or rebinding
   the canvas cancels the preview. The committed image is exact because
   `commit_move` runs the real renderer. **Remaining known cost:** the single
   ~39 ms commit composite on mouse-up (item 2's deep clone still applies to
   it). **Limitation:** the live preview is source-over only — non-Normal blend
   modes, layer masks, and clipping are not reproduced mid-drag.

2. **Deep document clone for history on every commit.** `record`
   (`cxxqt_object.rs:1332-1338`) calls `snapshot`, which does `doc.clone()`
   (`cxxqt_object.rs:1340-1347`) — a full copy of every layer's pixel buffers
   plus the composite. For a large document this is the dominant commit cost
   and doubles work on top of the double composite. Investigate a copy-on-write
   or delta snapshot for moves.

3. **`changed` fans out to every panel, whole-canvas work each time.**
   `addDocument` connects `PictureView::changed` to `PicturaMainWindow::refresh`
   (`frame.cpp:257`); `refresh` calls `retargetDock`, which calls
   `LayersPanel::refresh`, `NavigatorPanel::refresh`, `InfoPanel::refresh`, and
   `HistogramPanel::refresh` (`frame.cpp:555-579`). `LayersPanel::refresh`
   scales a thumbnail per layer via `layer_image` + `SmoothTransformation`
   (`layers_panel.cpp:354-389`); `HistogramPanel::recompute` does
   `convertToFormat(RGB32)` plus an O(canvas) per-pixel scan on the composited
   image (`histogram_panel.cpp:92-112`). This runs on every `changed`, including
   after every move commit and (potentially) during live preview if wired to
   `changed`. Investigate throttling/coalescing, per-panel change scoping, and
   decimated histogram sampling.

4. **No live preview in the Move tool — RESOLVED.** Originally
   `ToolController::handleMoved` for Move accumulated delta only
   (`tools.cpp:301-305`) and `translate_layer` ran on release
   (`tools.cpp:350-357`), so the layer jumped on mouse-up. Now the drag updates
   only a Qt-side delta over the cached preview (fix 1 under item 1 above) and
   `commit_move` runs the real renderer once on release; no per-event
   recomposite is added.

5. **Pan gating and missing gestures.** `ImageView::mousePressEvent` starts a
   pan only for `Qt::LeftButton` and only when `panEnabled_`
   (`image_view.cpp:141-151`); `panEnabled_` is only true for the Hand tool.
   Middle-button events fall through to the tool signal and are ignored
   (`tools.cpp:198-202` returns for non-left). No Space key handling exists.
   This is cause of problem 3, and Space+drag is a CS6 gap.

6. **Open view reset.** `setImage` zeroes the offset (`image_view.cpp:24-30`),
   producing the top-left placement in problem 2. `fitOnScreen` exists but is
   not called on open/new. `Frame::refresh` also calls `setImage` whenever
   `has_document()` is false (`frame.cpp:565`), so the reset can re-fire.

---

## 5. Acceptance checks

These are runnable by hand or as a small test harness with no framework
requirements. The Move-tool preview and history checks now have an automated
counterpart in the app self-test (exit codes 64–67; `m25_move preview=1 hist=1
undo=1` and `m25_preview_cache began=1 base=1 layer=1 hist_unchanged=1`).

1. **Open placement.** Open a PNG/PSD larger than the viewport. The image is
   fully visible and centred; the status-bar zoom is < 100%. Open a document
   smaller than the viewport: it is shown at 100%, centred (not top-left).
2. **Zoom commands.** `Ctrl+0` fits and centres; `Ctrl+1` shows 100% centred;
   `Ctrl++`/`Ctrl+-` step the preset ladder and disable at 3200% / minimum.
   Status bar, Navigator, and image stay in sync after each.
3. **Live move.** With the Move tool, drag a layer on a ≥ 24 MP document. The
   pixels follow the cursor continuously (no freeze until release). Release
   produces **exactly one** new History entry labelled "Move Layer"; no
   intermediate entries. `Ctrl+Z` reverts the whole move in one step. *(Resolved:
   the drag composites nothing — it draws a cached base plus the moved layer at
   the live delta; only mouse-up runs the real renderer once. Verified by
   `m25_move`/`m25_preview_cache` above.)*
4. **Move feedback.** With `Show Transform Controls` on, the bounding box and
   handles track the moving layer during the drag and match its final rect.
5. **Middle-drag pan.** With any active tool (Move, Brush, Marquee), middle-drag
   pans the canvas and the cursor shows a pan cursor. Document unchanged; no
   history entry. *(This is the [ext] check; it is expected to fail CS6 parity
   by design.)*
6. **Space+drag pan.** With any active tool, hold `Space`, drag, release:
   pans; releasing `Space` restores the previous tool; no document change and no
   history entry.
7. **Wheel zoom anchor.** Wheel-zoom over a document detail: the point under the
   cursor stays under the cursor.
8. **Frame budget.** Over a 5-second pan/zoom gesture on a 24 MP document,
   measure p95 frame time ≤ 20 ms (ARCH-013 acceptance 1). On the CPU fallback,
   ≤ 2× that.
9. **Move drag does not block.** During a 24 MP Move drag, log GUI-thread time
   per `mouseMoveEvent`; it stays ≤ 2 ms per event and no full-document
   composite runs per event (verified by instrumentation or sampling profiler).
   *(Resolved: a drag only updates the Qt delta over the cached preview; the
   single composite runs once on mouse-up — ~39 ms optimized, ~257 ms debug for
   a 1024×1024 two-layer document.)*
10. **History integrity.** Pan, zoom, Navigator drag, and tool switches add no
    History entries; only committed moves/edits do.
11. **Workspace snap.** With `View > Snap Canvas To Workspace` off, a fitted
    document pans and shows both scroll bars. With it on, a fitted document sits
    at the workspace centre, a middle- or Space-drag does not move it, and both
    scroll bars are hidden; zooming past the workspace restores panning and the
    bars.

---

## 6. Sources

Fetched or read for this note:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  CS6 Help (local extract `/tmp/opencode/photoshop_reference.txt`). Used for:
  Hand-tool/spacebar/Flick-panning (p. 113); Navigator proxy view area and
  `Ctrl/Cmd`-drag (p. 114); Zoom tool "next preset percentage", 3200% max /
  1-pixel min, `Zoom With Scroll Wheel`, `Zoom Resizes Windows`, Animated Zoom
  (pp. 114–115); Fit On Screen and pixel grid (p. 116); status-bar magnification
  and view options (p. 118); Move tool drag/nudge and `Show Transform Controls`
  (pp. 163–164, 167).
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/document-fit-on-screen-not-100/m-p/10874690`
  — "Photoshop by default opens at Fit to Screen or Ctrl/Cmd 0."
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/is-there-a-way-to-have-it-so-ps-does-not-open-with-the-default-fit-on-screen/td-p/14604011`
  — users ask how to disable the default Fit-on-Screen open view.
- `https://www.photoshopessentials.com/basics/how-to-use-the-navigator-panel-in-photoshop`
  — Navigator preview, view box, drag-to-pan, slider, `Ctrl`-drag view box.
- `https://helpx.adobe.com/photoshop/using/viewing-images.html` — Navigator
  interactions (`Window > Navigator`, proxy drag, `Ctrl`/`Cmd`-drag).
- `https://helpx.adobe.com/photoshop/using/tool-techniques/zoom-tool.html` —
  next-preset zoom, options bar (Fit Screen, Fill Screen, Resize Windows to Fit,
  Zoom All Windows, Scrubby Zoom).
- `https://helpx.adobe.com/photoshop/using/image-information.html` — status bar
  shows current magnification and file size.
- `https://community.adobe.com/questions-712/panning-with-the-middle-mouse-button-1126579`
  and `https://github.com/bigorados-bigo/PhotoshopMiddleMousePan` and
  `https://www.reddit.com/r/photoshop/comments/ttzfv/...` — Photoshop does not
  pan on middle-drag by default; users add it externally. Confirms the
  middle-drag pan is an extension.
- `http://www.photoshopforphotographers.com/CC_2013/Help_guide/tp/Move_tool.html`
  — a series of Move nudges counts as a single history step; Move options bar /
  Show Transform Controls.
- `https://photoshoptrainingchannel.com/tips/view-actual-pixels-fit-on-screen`
  — `Ctrl+1` = Actual Pixels, `Ctrl+0` = Fit on Screen.
- `https://photoshoptrainingchannel.com/tips/show-transform-controls` and
  `http://www.photoshopforphotographers.com/CC_2013/Help_guide/tp/Move_tool.html`
  — Show Transform Controls draws the bounding box and changes the options bar
  when a handle is grabbed.

Repository spec inputs read: `docs/03-tools/hand-and-zoom.md`,
`docs/03-tools/move-and-transform.md`, `docs/01-architecture/gpu-rendering-pipeline.md`,
`docs/01-architecture/performance-targets.md`,
`docs/01-architecture/threading-and-concurrency.md`,
`docs/01-architecture/rust-qt-interop.md`, `docs/02-ui-ux/application-frame.md`,
`docs/02-ui-ux/panels/navigator-panel.md`.
