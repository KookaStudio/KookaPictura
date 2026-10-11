# Tasks

## 1. Engine — document-scope arbitrary rotation

- [x] 1.1 Add `pictura_render::rotate_document_in(doc, angle_deg, pivot) -> bool` in `crates/pictura-render/src/document_ops/`, reusing the `pictura_ops::rotate_in` kernel; expand the document to the rotated bounding box and remap every layer rect, mask rect, vector-mask rect, smart-object bound, and document channel, supporting every layer kind including Background; angle 0 is unchanged and a non-finite/out-of-range angle or missing document returns `false` without mutating. Verify with inline unit tests covering the right-angle equals-`rotate_document`, kind-matrix, refusal, and angle-0 cases (`cargo nextest run -p pictura-render`).
- [x] 1.2 Add the `rotate_document_in` oracle to `crates/pictura-render/tests/document_oracle.rs`: right angles bit-exact against `rotate_document`, arbitrary angle within the tolerance already recorded for `rotate_in` in `pictura-ops/tests/oracle.rs`, self-skipping when `magick` is absent and not `#[ignore]`d. Verify the whole `document_oracle` suite passes with and without `magick` on PATH.
- [x] 1.3 Wire `Image > Image Rotation > Arbitrary` through the command registry: a validated numeric angle, rotate about the document centre via `rotate_document_in`, recomposite, and record exactly one history state; invalid input records nothing. Verify with a self-test (or Qt Test) check and the app `--self-test` run.

## 2. Straighten crop — model, gesture, snapping, shield

- [x] 2.1 Extend `CropToolHandler` (`cpp/tool_crop.cpp`) and the crop commit path to carry a straighten angle: commit composes `rotate_document_in` with `crop_document` and still records one `Crop` state; angle 0 keeps today's behavior. Verify with a `selftest_crop_tool.cpp` check (rotated box commit produces the box-sized rotated content) and `./build/pictura --headless --self-test`.
- [x] 2.2 Extract Free Transform's outside-corner rotate band into a shared helper and use it in Crop: press inside the band enters rotate about the box centre, beyond the band draws a new box, Shift snaps to 15°, and the cursor switches. Verify with a Qt Test for the band/new-box/hit-test behavior (`ctest --test-dir build -R '^tst_'`).
- [x] 2.3 Add crop snapping: while resizing/moving, snap the dragged edge/corner to the canvas or a layer's bounding edge within a screen-pixel threshold, honoring an active aspect ratio. Verify with a Qt Test that a near-canvas and a near-layer drag land exactly on the candidate and an out-of-threshold drag does not snap.
- [x] 2.4 Fix the crop shield to dim only the canvas region outside the box (replace `viewRect()` at `image_view_overlays.cpp:119` with the canvas bounds) and follow the box as it rotates. Verify with a Qt Test asserting workspace pixels outside the canvas are undimmed.

## 3. Preview — content rotation with screen-aligned overlays

- [x] 3.1 Add a content-rotation preview to `ImageView` (angle + pivot) that rotates the drawn composite and expands the clip past the original canvas so rotated corners are not cut, sourcing from the view-pyramid level and caching per `(angle, docRect)`. Verify with a Qt Test/golden check that rotated corners render and the full-res resample is not run per frame.
- [x] 3.2 Route crop overlays through a screen-axis-aligned path so the box, handles, and rule-of-thirds guides never tilt under content rotation. Verify with a Qt Test that the drawn crop box stays axis-aligned at a non-zero straighten angle (or with a seeded angle, exercising the real straighten path).

## 4. Modal tool-session undo/redo

- [x] 4.1 Add `toolUndo()`/`toolRedo()` (default `false`) to `ToolHandler` and make the Edit Undo/Redo handlers (`frame_menus.cpp`) consult the active tool first, falling back to the document history, with the menu enabled/label providers reflecting the active source. Verify with a Qt Test: a session with steps undoes the session, an empty session falls back to the document history, and Cancel discards the session.
- [x] 4.2 Implement the Crop session: push `(box, angle)` on each drag release that changes state, Ctrl+Z/Ctrl+Shift+Z walk it while active, Cancel/Escape discards it, and Apply collapses it to exactly one `Crop` state. Verify with a Qt Test for record/undo/redo/discard/collapse.

## 5. Crop options bar

- [x] 5.1 Rebuild `OptionsBar::buildCropPage` with the CS6 controls — ratio select with `New Crop Preset...`/`Delete Crop Preset...`, `W`/`H` via the shared `NumericField`, swap-aspect, `Clear`, `Straighten` + spirit-level, grid menu, cog menu, and `Delete Cropped Pixels` — wiring presets/fields/straighten to the crop model. Register any new `.cpp`/`.h` in `CMakeLists.txt`. Verify with a Qt Test: preset locks the ratio, a user preset is added then deleted, `W`/`H` edit the box, and `Straighten` sets the angle.

## 6. Integration verification

- [x] 6.1 Run `bash scripts/verify-full.sh` and `ctest --test-dir build -R '^tst_'`, confirm the file-size allowlist and milestone-name guards pass, and update `docs/dev/STATE.md` (carrying `TASK-ALLOWS-DOCS`) if the change lands on a milestone boundary.
- [x] 6.2 Run `openspec validate --all --strict` and confirm the `crop-straighten` change validates clean.

## 7. UX review corrections (#252)

- [x] 7.1 Preview: the canvas grows to the rotated content's axis-aligned bounding box and the content is not clipped at the old canvas edges; the straighten pivot is fixed at the box centre when the angle is set so moving the box leaves the composite in place (`image_view.cpp`, `image_view_overlays.cpp`, `tools.*`).
- [x] 7.2 Rotate cursor/gesture: outside the crop box (beyond an 8px resize margin, inside the canvas) always rotates; the crosshair/new-box mode is only outside the canvas; the margin keeps the rotate zone from fighting the resize handles (`crop_grip.h`, `tool_crop.cpp`).
- [x] 7.3 Crop options-bar icons use Lucide SVGs (swap `arrow-left-right`, spirit-level `angle`, grid `grid-3x3`, cog `settings`, reset `undo-2`, cancel `ban`, apply `check`) instead of text glyphs (`options_bar.cpp`, new `crop.*` assets + `lucide-map.json` + `pictura.qrc`).
- [x] 7.4 Crop sub-history refreshes the Edit Undo/Redo enabled state on every session change, so Ctrl+Z reaches the session when the document history is empty (`ToolController::notifyToolSessionChanged`, `frame_build.cpp`).
- [x] 7.5 Cancel/Escape clears the crop box and angle, discards the session, and stays in the Crop tool with no box drawn, so a fresh drag draws a new box honoring the ratio; the straighten commit uses the stored pivot (`tool_crop.cpp`, `crop_group.rs`).
- [x] 7.6 Qt Test coverage for the canvas growth, fixed pivot, cancel→init mode, and command-level session undo (`tst_crop_preview.cpp`, `tst_crop_session.cpp`, `tst_crop_grip.cpp`).

## 8. Crop cursor model, Classic/Modern mode, persistence (#252)

- [x] 8.1 Fix the crop pointer cursors: resize over a handle, the workspace default arrow over the box body, rotate outside the box, new-crop crosshair outside the canvas / in init mode; a framework `ToolHandler::hoverCursor` hook so modifier keypresses, tool switches, and canvas rebinds do not revert the cursor (`tool_handler.h`, `tool_context.h`, `tools_marquee.cpp`, `tools.cpp`, `tool_crop.cpp`).
- [x] 8.2 Add the workspace default arrow cursor (`assets/cursors/cursor.workspace.svg`, the bare tool-arrow) and set it as the canvas default; panels/menus/dialogs keep the system cursor (`image_view.cpp`, `pictura.qrc`).
- [x] 8.3 Classic (default) vs Modern crop drag semantics behind a `Use Classic Mode` cog-menu toggle: Classic drags move the box, Modern drags pan the content under the fixed box (`tool_crop.cpp`, `image_view.cpp`, `options_bar.cpp`, `crop_group.rs` unchanged).
- [x] 8.4 Persist the crop options (mode, grid overlay, ratio, Delete Cropped Pixels) in the session store and restore them at startup (`session.h`, `session.cpp`, `frame.cpp`, `frame_session.cpp`, `frame_build.cpp`).
- [x] 8.5 Qt Test coverage for the cursor zones, Classic vs Modern drag, and option persistence (`tst_crop_cursor.cpp`, `tst_crop_options.cpp`).

## 9. Crop cursor/menu/backdrop follow-up (#252)

- [x] 9.1 Correct the workspace default cursor to the described arrow (tip top-left, vertical left edge, up-right barb, 0° notch, 45° edge back to the tip) and rebuild every tool cursor on it (`cursor.workspace.svg`, tool cursor assets).
- [x] 9.2 With an active box, everything outside it rotates (including past the canvas edge); the new-crop cursor only appears in init mode (`crop_grip.h`, `tool_crop.cpp`).
- [x] 9.3 Rework the crop options bar into `options_bar_crop.cpp`: the grouped ratio menu with a `W x H x Resolution` mode (px suffix + resolution field/unit only there), the full grid menu (overlays, show mode, cycle hints), and the full cog menu (Classic toggle, shield colour/opacity, display toggles).
- [x] 9.4 Classic starts boxless and Modern starts with the full-canvas box on first selection (`tool_crop.cpp`).
- [x] 9.5 Canvas backdrop: fill the expanded/rotated area with the background colour when the document has a Background layer, else the transparency checkerboard (`image_view.cpp`, `tools.cpp`).

## 10. Crop surface parity (#252)

- [x] 10.1 Fill the shared tool-arrow black beneath its white outline in every cursor built on it — `tool.move`, `tool.lasso`, `tool.magneticlasso`, `tool.patch`, `tool.polygonallasso`, `tool.contentawaremove`, `cursor.moveSelection` — leaving the rest of each cursor's art white (`assets/cursors/*.svg`). Verify by rendering and the `svg_cursors` scenario.
- [x] 10.2 Lift the heading→rotated `cursor.rotate` helper into `icons.{h,cpp}` and give the crop rotate cursor eight zoned orientations (four corner diagonals, four edge axes) with the corner zones widened so the corner variant is reachable. Verify with a Qt Test over the eight zones (`tst_crop_cursor.cpp`).
- [x] 10.3 Clamp the first box drawn in the init mode to the canvas; allow a later handle resize past the canvas and grow the displayed canvas to contain the box (`tool_crop.cpp`, `image_view_overlays.cpp`). Verify with a Qt Test.
- [x] 10.4 Add the non-clamping grow mode to `crop_document`: committing a box larger than the canvas grows and pads it — background colour with a Background layer, transparent otherwise — while the clamp path stays for `Image > Crop` (`crates/pictura-render/src/document_ops/crop.rs`, `crop_group.rs`). Verify with a Rust unit test and the crop-growth Qt Test.
- [x] 10.5 Remove the Modern content-pan; both Classic and Modern drag the crop box, differing only in the initial box (`tool_crop.cpp`, `image_view.{h,cpp}`, `tools.{h,cpp}`). Verify with the updated Classic/Modern Qt Test.
- [x] 10.6 Crop options bar: `Ratio` first and default, `W`/`H` aspect-ratio values in ratio mode (pixel width/height plus resolution only in `W x H x Resolution`), vertical-rule group separators, a disabled `Content-Aware` placeholder, and the spirit-level straighten toggle (`options_bar_crop.cpp`, `options_bar.h`). Verify with `tst_crop_options.cpp`.
- [x] 10.7 Straighten line gesture: arming the spirit-level toggle and dragging a line sets the crop angle to the line's inclination and disarms; preview the line in `ImageView` (`tool_crop.cpp`, `tools.{h,cpp}`, `image_view_overlays.cpp`). Verify with a Qt Test.
- [x] 10.8 Verify `openspec validate --all --strict`, run `bash scripts/verify-full.sh`, rebuild `build/pictura`, and update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).

## 11. Crop corrections (#252)

- [x] 11.1 Add the `preview` crop state: `ImageView::setCropPreview(bool)` (dashed outline, no guides, no handles) and a `none → preview → active` model in `CropToolHandler`; Modern starts with a centered, ratio-fitted preview box, Classic starts boxless; dragging inside the preview draws a new box, a click adopts it, a rotate press activates it; ESC / Cancel return to `none`. Verify with Qt Tests over preview rendering, click-adopts, drag-draws, and ESC in both modes.
- [x] 11.2 Modern active drag pans the composite under a fixed box via a session content offset (`ImageView`), commit maps `box − offset`; Classic drag unchanged. Verify with a Qt Test (Modern drag keeps the box fixed on screen and moves the content; Classic moves the box).
- [x] 11.3 Make the paint canvas frame follow the box at every angle: use `cropCanvasImageRect()` whenever a box exists (not only straightening) so a box moved past the canvas pads with the backdrop/checkerboard instead of showing the workspace. Verify with a Qt Test pixel-asserting the padded area is the background colour (Background layer) and the checkerboard (no Background).
- [x] 11.4 Pin the draw-new press corner: anchor the fresh-draw ratio fit on the press point for any drag direction (`tool_crop.cpp`, `crop_grip.h`). Verify with a Qt Test dragging up/left with a ratio set and asserting the press corner is unchanged.
- [x] 11.5 Options-bar fields: drop the `W`/`H` labels and slider popup, accept decimals with integer-by-default formatting, show pixel dimensions plus a resolution field (no `Res:` label) in `W x H x Resolution` mode, size the `px/in`·`px/cm` unit combo to contents, and default the resolution to `document_ppi(view)`. Verify with `tst_crop_options.cpp`.
- [x] 11.6 Show Cancel/Apply (and Reset) only while the crop is active; add a `cropActive` query and re-sync the page on the crop-state signal. Verify with a Qt Test asserting the buttons are hidden in `none`/`preview` and shown when active.
- [x] 11.7 Straighten line gesture activates from `none`/`preview` on press (Classic: full canvas, Modern: the preview box), previews the rotation from the first drag frame, and updates instantly on release. Verify with Qt Tests.
- [x] 11.8 Update the existing crop Qt Tests to the new model and add coverage for the padded-move backdrop, the pinned start corner, and preview→active transitions; run `ctest --test-dir build -R '^tst_crop'`.

## 12. Nested-layer targeting (#252)

- [x] 12.1 Resolve the active-layer path to a leaf via `pictura_render::resolve_path(_mut)` in `helpers.rs` `active_pixel_layer`/`active_pixel_layer_mut`; reject a resolved group as a paint/fill/filter target. Verify with a Rust unit test painting a layer under a folder and a refusal for a group.
- [x] 12.2 Convert the secondary top-level `usize` parsers to path-based lookups: `impl_filters.rs`, `impl_transform/{mod,session}.rs`, `impl_selection.rs`. Verify with the affected suites.
- [x] 12.3 Convert the C++ top-level paths (`tools_marquee.cpp` `activePixelLocked`; `control_server_actions.cpp` lock/kind) to path-based bridges. Verify with a Qt Test that the brush paints a nested layer and the cursor/lock reflect it.
- [x] 12.4 Confirm composite-time ancestor semantics (group mask/clip/blend/opacity; Pass Through vs isolated) and record any gaps (group transform, Pass Through) as follow-ups rather than assuming them.

## 13. Layers panel parity (#252)

- [x] 13.1 Hover pointing-hand cursor on rows (viewport mouse tracking + `eventFilter` move/leave).
- [x] 13.2 Single-click lock removal for regular layers; shrink the row lock badge ~33%; shrink header lock buttons to the filter-bar icon size; narrow the blend-mode select.
- [x] 13.3 Adding a mask to a Background layer converts it (removes the lock).
- [x] 13.4 Recompute mask/vector thumbnails and link glyphs left-to-right from the image thumbnail with `link-2`/`unlink-2` assets; aspect-correct mask thumbs.
- [x] 13.5 Active-thumb brackets and thumb-click activation; Alt+click activates in the item and the composite.
- [x] 13.6 Folder rows: no brackets around the group icon, 50% smaller glyph, reduced right-column padding (child eyes unaffected).
- [x] 13.7 Make the visibility toggle responsive (patch the row in place rather than a deferred full model reset).
- [x] 13.8 Give a group row a chevron so an empty/new nested folder is expandable (`impl_layers.rs`); decide the `group_paths` cross-container rule.
- [x] 13.9 Qt Test coverage for the new cursors, lock toggle, thumb layout, active thumb, folder rows, and nested-group chevron; add the `link-2`/`unlink-2` assets via `lucide-map.json` + `pictura.qrc`.

## 14. Channels panel parity (#252)

- [x] 14.1 Hover pointing-hand cursor on channel items.
- [x] 14.2 Ctrl shows a `select.all` dashed-square overlay at the cursor.
- [x] 14.3 Restyle rows to match the Layers panel table/item style (`Theme::shade` surfaces, eye/thumb layout).
- [x] 14.4 Qt Test coverage for the hover cursor and Ctrl overlay.

## 15. Options bars — separator, Move, Eyedropper (#252)

- [x] 15.1 Add a separator after the tool icon for every active tool page via a shared helper in `OptionsBar::buildPage`. Verify with a Qt Test iterating the pages.
- [x] 15.2 Move bar: Auto-Select (Group/Layer), Show Transform Controls, and a three-dots menu with align/distribute plus `Align To:` Selection/Canvas (`align_apply` gains a Canvas branch). Verify with Qt Tests.
- [x] 15.3 Eyedropper: add `EyedropperOptions` (Sample Size Point…101×101, Sample scope, Show Sampling Ring), a new options page, and a size/scope-aware `sample_argb_*` bridge. Verify with Qt Tests.

## 16. Select tools Alt two-stage (#252)

- [x] 16.1 Make `marqueeDragRect` take an explicit mirror flag; track an Alt edge during the drag so the first Alt subtracts and a second Alt mirrors the pivot. Update the `runSelfTest` selection checks and add a Qt Test.

## 17. Canvas pan and tool-switch zoom (#252)

- [x] 17.1 Suppress tool overlays (the brush ring at least) while `panning_ || spacePan_`, and repaint on `setSpacePan` and middle press. Verify with a Qt Test.
- [x] 17.2 Preserve canvas zoom/offset on a tool switch: only `applyInitialView()` on the first sizing after a document load. Verify with a Qt Test that switching tools keeps zoom and position.

## 18. Info and Histogram panels (#252)

- [x] 18.1 Info: one hint per line, capitalise the first letter of each, reduce the font by 2px. Fix the icon-menu first-open clipping (`ensurePolished`/`adjustSize` before reading `sizeHint`). Verify with Qt Tests.
- [x] 18.2 Histogram: add a default `All Channels` entry rendering the R/G/B overlay (extend `HistogramView` to multiple series). Verify with a Qt Test.

## 19. Footer chevrons (#252)

- [x] 19.1 Replace the painted chevrons with tinted Lucide icons; add `chevron-up`/`chevron-left` assets via `lucide-map.json` + `pictura.qrc` + the sync script. Keep the existing test hooks. Verify with a Qt Test.

## 20. Integration verification

- [x] 20.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/verify-full.sh`, and `ctest --test-dir build -R '^tst_'`; confirm the file-size allowlist and milestone-name guards pass.
- [x] 20.2 Run `openspec validate --all --strict`, rebuild `build/pictura`, and update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`) to record this audit pass.
