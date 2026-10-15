# Proposal: type-tool-session-reachthroughs

## Why

Issue #358. `ToolHandler` (`crates/pictura-app/cpp/tool_handler.h`, 42 virtuals)
mixes a genuinely generic surface (pointer, activate/deactivate,
options-changed, document-refreshed, session undo, annotations clear) with tool-
specific reach-throughs — crop box (10), perspective crop (3), type (4), and the
keyboard-gesture quartet (4). Every reach-through is re-exposed as a thin
`ToolController` trampoline (`registry_.forTool(id)->virt()`) before reaching its
real callers — typically 1–2 production files. The fat base makes each of ~35
handlers implement a scattered subset of an interface that mostly does not apply
to it.

Investigation also shows the issue's "lasso family" is misclassified: the four
methods grouped under it are generic keyboard-session hooks, not lasso
operations — `cancelPolygonLasso` is the Esc-cancels-the-active-gesture hook
(overridden by 7 unrelated tools), `removeLassoPoint` the Delete hook (3 tools),
`commitPolygonLasso` the Enter hook (Pen + 2 lassos), and `lassoInProgress` a
selection-routing predicate (1 tool). Typing them as a "lasso" interface would
force Slice, RotateView, PathSelection, Crop, and Perspective Crop to pretend to
be lassos.

## What Changes

- Shrink `ToolHandler` to the generic surface only: pointer press/move/release,
  activate/deactivate, document-refreshed, options-changed, the pointer-cursor
  hooks (`pointerCursor`/`hoverCursor`/`pointerCursorKind`/`cursorVariant`), the
  arrow-key `nudge`, the marquee test hooks (`dragMods`/`marqueeMirror`), the
  generic session undo/redo (`toolUndo`/`toolRedo`/`canTool*`), and
  `clearAnnotations`.
- Introduce narrow **session ports** in a new `tool_sessions.h`:
  - `CropCommitSession` — `commitCrop()`; **shared** by Crop and Perspective
    Crop (both implement it).
  - `CropBoxSession` — Crop only: `cropActive`, `resizeCrop`, `cropWidth`,
    `cropHeight`, `resetCropBox`, `cropCenter`, `setStraightenMode`,
    `hasPendingCrop`, `pendingCropRect`.
  - `PerspectiveCropSession` — Perspective Crop only: `setPerspectiveOutput`,
    `perspectiveQuadComplete`, `setPerspectiveShowGrid`.
  - `TextSession` — the active type tool: `commitText`, `cancelText`,
    `textActive`, `insertText`.
- Add typed resolvers on `ToolController`: `cropCommitSession()`,
  `cropBoxSession()`, `perspectiveCropSession()`, `textSession()`, each
  resolving the port with `dynamic_cast` over `handlerFor(id)`, mirroring the
  shipped `optionsFor<T>(ToolId)` pattern from `tool-options-handler-seam`
  (#357). `cropCommitSession()` is the single place the active-vs-fixed ToolId
  choice lives (it preserves `tools.cpp`'s `active_ == PerspectiveCrop ?
  PerspectiveCrop : Crop`); `cropBoxSession()` and `perspectiveCropSession()`
  resolve a fixed `ToolId`; `textSession()` resolves the active tool.
- Reclassify the generic gesture quartet: rename `commitPolygonLasso` →
  `onFinishPath`, `cancelPolygonLasso` → `onEscape`, `removeLassoPoint` →
  `onDeletePoint`, `lassoInProgress` → `gestureInProgress`, keeping them on
  `ToolHandler` and keeping the `bool` consumed/passthrough contract the frame
  relies on. Clean `frame.cpp`'s hardcoded `Enter -> {PolygonalLasso,
  MagneticLasso, Pen}` list and its Esc/Delete routing to dispatch through the
  renamed hooks.
- Delete the crop, perspective, and type virtuals from `tool_handler.h`, their
  trampolines from `tools.{h,cpp}`, and the crop-family aliases that just forward
  to the generic hooks (`cancelCrop` == Esc; `resetCrop` ==
  `CropBoxSession::resetCropBox`). Retarget the controller's own internal crop
  callbacks (`tools.cpp` `cropCenter`/`setStraightenMode`) and the inline
  `tools.h` `hasPendingCrop`/`pendingCropRect` onto `cropBoxSession()`.
- Retarget the callers mechanically: `frame.{h,cpp}`, `frame_menus_image.cpp`,
  `frame_menus_edit.cpp`, `options_bar_crop.cpp`, `options_bar_type.cpp`,
  `panels/glyphs_panel.cpp`, plus the affected test/self-test files.
- No user-visible behavior change: same outcomes, same Qt and self-test suites.
  One routing nuance: Enter is now consumed only when the active tool reports it
  finished a path (`onFinishPath`), instead of unconditionally for the old
  hardcoded `{PolygonalLasso, MagneticLasso, Pen}` list. No Enter binding is
  reachable for an idle gesture tool, so nothing observable changes.

Explicitly out of scope: moving crop/text session *state* into a
controller-owned store (the follow-up the port seam makes tractable), and Free
Transform (not a `ToolHandler` at all).

## Capabilities

No spec delta: internal seam refactor with no requirement change
(`skip_specs: true` in `.openspec.yaml`). The behavioral contract is unchanged,
so no `openspec/specs/` file changes.

## Impact

- New: `crates/pictura-app/cpp/tool_sessions.h`.
- `crates/pictura-app/cpp/tool_handler.h` (shrinks), `tools.{h,cpp}`
  (trampolines deleted, typed resolvers plus internal-callback and inline
  `hasPendingCrop`/`pendingCropRect` retargets), `tool_crop.cpp`,
  `tool_perspectivecrop.cpp`, `tool_type.cpp`, `tool_selection.cpp`,
  `tool_magneticlasso.cpp`, `tool_pen.cpp`, `tool_slice.cpp`,
  `tool_rotate_view.cpp`, `tool_path_selection.cpp`.
- Callers: `frame.{h,cpp}` (`hasPendingCrop`/`commitCrop` wrappers and the
  Enter/Esc routing), `frame_menus_image.cpp`, `frame_menus_edit.cpp`,
  `options_bar_crop.cpp`, `options_bar_type.cpp`, `panels/glyphs_panel.cpp`.
- Tests/self-tests retargeted onto the ports: `tst_crop_options`, `tst_crop_snap`,
  `tst_crop_preview`, `tst_crop_cursor`, `tst_type_panels`, `tst_type_tools`,
  `selftest_magnetic_lasso`, `selftest_tools_selection`, plus a new
  `tst_session_ports` Qt Test for port resolution. (`tst_crop_session` and
  `selftest_crop_tool` use only the retained `hasPendingCrop`/`pendingCropRect`
  and are unchanged.)
- `CMakeLists.txt` must list the new `tool_sessions.h` (headers are listed
  explicitly, no globbing).
- No new dependency.
