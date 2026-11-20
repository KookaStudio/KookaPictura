# Design

## Context

See `proposal.md` for motivation and `specs/tools/refine-edge/spec.md` for the
behavior contract.

The shipped feature is a dialog over a stateless preview: `refine_edge_preview`
runs the whole engine on a 220 px proxy, `refine_edge_apply` reruns it at
document size, and `RefineEdgeDialog` only ever changes the document on OK.
The canvas already has two dialog bridges that this change extends rather than
replaces:

- `runDialog` blocks the parent window, but `ToolController::setCanvasSampler`
  plus `setDialogCanvasSampling` let a dialog receive left-button canvas input
  (the Color Range / Replace Color eyedroppers).
- `ImageView` already draws transient document-anchored overlays
  (`setFilterPreview`, `setMaskOverlay`) and a brush-size ring.

`refine.rs` computes one band from `settings.radius`; the identity early-return
is what makes the default dialog a no-op. The brush disables that early-return
when any dab is present (see Decisions).

## Goals / Non-Goals

**Goals:**

- Local, ordered band overrides without disturbing the existing global
  pipeline or the `refine(mask, image, settings)` contract.
- Brush interaction on the main canvas for both the menu's Refine Edge and the
  Layers panel's Refine Mask.
- Accurate proxy preview: dabs scale like the other pixel distances.
- Test the dialog and canvas state through `tst_refine_edge` accessors, not
  manual hover checks.
- Stay within the file-size caps (notably `tools.cpp` 1162/1200,
  `image_view.cpp` 1195/1200, `layers_panel.cpp` allowlisted at 1262).

**Non-Goals:**

- A Select and Mask workspace shell, Onion Skin, or canvas view-mode rendering.
- The mask-painting Brush tool, Refine Hair, Color/Object Aware modes, High
  Quality Preview, or settings persistence.
- Changing the existing global refinements or decontamination.

## Decisions

### Ordered dabs in the engine, not a rasterized C++ mask

`RefineDab { x, y, radius, erase }` is passed as an ordered list. Strokes are
recorded in document pixels; the bridge scales `x` by the proxy's x ratio, `y`
by its y ratio, and the radius by the larger ratio before refining, and the
engine applies each dab in the pixel space of the coverage it is given (proxy
for the preview, document for the commit). The engine rasterizes a signed disk
field with a per-pixel local radius and applies dabs in order, so
refine→restore→refine behaves as CS6/CC do. Alternatives: a document-sized pair
of masks (order lost, up to 2 bytes/pixel, and a proxy mask would have to be
upscaled before commit); a single global brush radius (cannot survive resizing
between strokes). The chosen field is 5 bytes/pixel transient and only built
for a brushed run; a radius-only refine allocates none of it.

The local rule is `effective = max(settings.radius, dab.radius)` for refine
dabs; restore dabs write the input coverage. Each dab overwrites the local
refine/radius state in its disk in order: a later refine dab replaces an
earlier refine dab's radius, and a later restore dab clears refinement,
including band expansion; `edge_band_from` does not mark a pixel last restored.
The band's existing distance precheck becomes `dist > max(radius, max valid dab
radius)`, so unbrushed pixels still read no image. The identity early-return is
disabled when any dab is present, and the band step runs when
`settings.radius > 0 || !dabs.is_empty()`, so a refine dab with Radius 0 still
refines. `refine` keeps its signature and passes no dabs; `refine_from` gains
the list. `edge_band_from` shares the same per-pixel radius helper for Show
Edge. `lib.rs` exports `RefineDab` and `edge_band_from`.

Dabs are ignored when any coordinate or radius is non-finite, the radius is
negative, the centre lies outside the coverage, or the radius exceeds the
coverage's longer side; ignored dabs never produce output. The disk is
hard-edged; anti-aliased dab edges are a ceiling because the band blend
already softens the result.

### Wire format: flat `f64` quads

The bridge accepts `(x, y, radius, erase)` as `&[f64]`, matching the existing
`::rust::Slice<const double>` pattern (`perspective_crop_commit_sized`). The
preview path scales `x` by `image.width / doc.width`, `y` by
`image.height / doc.height`, and the radius by the larger ratio, so the rounded
proxy axes stay aligned with the per-axis mask sampling; Radius/Smooth/Feather
keep using `preview_scale(doc, proxy)`. Apply uses the dabs unchanged.
`[re_brush_scale]` allows two proxy pixels and uses a dab radius of at least 8
document pixels; `scale_preview_params` already clamps non-zero distances to at
least one proxy pixel. Band previews are separate externs beside the existing preview pairs,
returning one byte a proxy pixel.

Changing `refine_from` updates `crates/pictura-select/src/tiled/tests.rs:372`,
`crates/pictura-app/cpp/tests/tst_refine_edge.cpp:177,223,229`,
`crates/pictura-app/cpp/refine_edge_dialog.cpp:214,216`,
`crates/pictura-app/cpp/menu_select.cpp:221-224`, and
`crates/pictura-app/cpp/panels/layers_panel_mask_menu.cpp:170-172`; each
passes an empty slice in the same step. Verify with
`cmake --build build --target pictura`, because `cargo test --no-run` does not
build the C++ call sites.

### Canvas input through the tool controller, not a dialog widget

A new `ToolController::setCanvasBrush(handler)` forwards
`Hover/Press/Drag/Release` with the pointer in image space. It reuses the
input-blocker exception pattern: `setDialogCanvasBrush` lets only left-button
and hover events over an `ImageView` through while armed. The hook also takes
the cursor (`CrossCursor`), disables pan while armed (the Hand tool would
otherwise pan instead of brushing), and refuses the ring while
`spacePan`/panning owns the cursor. Alternatives: a paint widget inside the
dialog (contradicts the documented canvas surface and loses zoom/precision),
or application-wide mouse filters (wrong ownership).

The dialog resolves `ToolController*`/`ImageView*` through a small
composition-root provider installed by `MenuRegistrar`, mirroring
`setDialogNavigationKeys`. That keeps the Layers panel path working without
growing `layers_panel.cpp` (which is at its allowlist ceiling).

### Preview overlay as a separate `ImageView` surface

`setRefinePreview(QImage, QRect)` draws above the document, after the filter
and mask overlays. The dialog composes one ARGB proxy image: a 1 px red
outline where the refined coverage crosses 128, plus a green tint over the
Show Edge band. Reusing `setMaskOverlay` would collide with the rubylith
subsystem (its cached key and frame resync), and animated ants would need a
contour bridge call per stroke. A static outline is the lazy ceiling; the
thumbnail still shows the full coverage.

The dialog exposes `dabsForTest()`; `ImageView` exposes
`refinePreviewForTest()` and `hasRefinePreviewForTest()`, so `tst_refine_edge`
verifies the overlay and ring without manual hover checks.

### Brush UI and keys

The brush is armed for the dialog's lifetime, as both generations do. Alt is
momentary restore: per dab,
`restore = latched || QGuiApplication::keyboardModifiers().testFlag(Qt::AltModifier)`;
releasing Alt returns to refine unless `Shift+E` latched restore. `Shift+E`
latches restore for CS6. The keys are `QShortcut`s with
`Qt::ApplicationShortcut` because clicking the canvas can move focus off the
dialog and the input blocker swallows keys addressed to the parent window.
Brush size is a diameter in document pixels, default 30, range 1–1000, bracket
steps a ±10 % round, session-only; the dab radius is size / 2, the hover ring
diameter is size, and dabs space at radius / 4. Dabs are appended on press and
on drag when the pointer has moved at least a quarter of the dab radius.

## Risks / Trade-offs

- Weak estimator → brushing can look subtle: the existing `estimate_coverage`
  only nudges coverage toward mid. The docs' strand-recovery parity criterion
  is not met by this estimator; the change records the geometry/approximation
  ceiling in `docs/08-selection/refine-edge.md`, `docs/dev/STATE.md`, and the
  SEL-4 row instead of claiming it.
- Proxy-resolution overlay → soft outline at high zoom: same accepted ceiling
  as the mask-overlay and filter-preview surfaces; the commit is exact.
- Hard dab edges → a visible local band boundary is possible on extreme
  settings: softened by the band's distance blend; anti-aliased dabs and
  hardness are a later upgrade.
- Canvas focus vs. shortcuts → application shortcuts can shadow tool keys
  while the dialog is open: the dialog is input-modal by design, and the
  dialog is destroyed on close.
- File-size pressure → `tools.cpp`, `image_view.cpp` have little headroom:
  hook implementation lives in `tools_marquee.cpp`, the overlay paint body in
  `image_view_document.cpp`, and the only additions to the capped files are
  the early dispatch branches / one paint call.
- `refine.rs` grows tests and brush code → under the 1200 cap by estimate; if
  it crosses, split the brush field into `refine_brush.rs` along the seam.
