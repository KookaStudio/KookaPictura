# Design

## Context

See `proposal.md` for motivation and the four capability deltas for the behavior contract.

Observed integration points:

- `cpp/toolbox.cpp` creates `paintMaskButton` and emits `paintMaskToggled`; the existing shell spec explicitly requires a stub. That requirement must be replaced.
- `pictura-select/src/lib.rs` stores coverage as `Selection { width, height, data: Plane<u8> }`. `DocumentSession` owns `Option<Selection>`, history, and one `ActiveSession` for an in-progress gesture.
- `helpers::active_mask_target`, `impl_paint::begin_paint`, `paint_tools`, and `impl_filters` already route layer-mask edits to grayscale painting/filtering. The layer-mask tests show that Eraser paints the background color and unsupported Healing/Pattern Stamp commands refuse.
- `cpp/image_view_document.cpp` has a view-only mask-overlay facility, and the mask-solo presentation path already crops a separate source through the view pyramid. Composite readers must remain distinct from the canvas source.
- `ChannelsPanel` currently distinguishes color rows, layer-mask rows and durable alpha rows; a temporary Quick Mask row needs its own identity instead of an alpha index.
- The existing `SessionState` JSON store holds application preferences. Layers-panel row painting uses shared theme colors; Quick Mask needs a transient active-row state and a theme token, not a stored layer color label.

The graph index did not cover this worktree. These findings were checked directly in the relevant source and nearby tests; no exhaustive call-graph claim is made.

The user clarified entry, exit, missing-layer blocking, and the red active-layer highlight on 2026-11-07. `docs/08-selection/quick-mask.md` and `docs/03-tools/quick-mask-tool.md` supply the other behavior. Their undo descriptions differ: this proposal chooses the tool document's per-edit history and no toggle states. That choice is a project contract, not a claim of independently measured CS6 history parity.

## Goals / Non-Goals

**Goals:** keep one authoritative coverage plane, reuse grayscale editing and selection history, keep canvas presentation out of document/composite data, and preserve the existing copy-on-write and bounded-view presentation mechanisms.

**Non-Goals:** native-depth selection storage, new color-dependent retouch algorithms, arbitrary alpha-channel editing, PSD Quick Mask metadata, a new GPU path, or fixing the separate channel-cap and Color Range preview gaps tracked by #614 and #613. Quick Mask supports the existing grayscale-compatible mask operations; unsupported tools refuse. No new dependencies or Adobe assets are needed.

## Decisions

### 1. Coverage means selected; protection is its inverse

Use the existing coverage convention throughout: `c = 255` means fully selected and `c = 0` means protected. The user's 0% interior / 100% exterior description is the protection amount `p = 1 - c / 255`, not a request to invert stored selection data.

For Masked Areas, display alpha is `opacity * (1 - c / 255)`. Selected Areas uses `opacity * c / 255`. The tint is a UI color independent of document color mode. A grayscale display shows `c` for Masked Areas and `255 - c` for Selected Areas. Convert foreground/background through the existing mask-luma approximation without reversing the paint value. Capture polarity at gesture creation: the grayscale editing destination is `c` for Masked Areas and `255 - c` for Selected Areas. Run the existing blend engine, including opacity/flow and supported blend modes, in that working convention; convert every preview and committed result back to canonical coverage (`c = result` or `c = 255 - result`). Brush, Pencil, Eraser, Paint Bucket and Gradient share these boundaries. Filters receive the same working convention and convert their preview/apply results back through the same boundary. Switching polarity changes neither stored coverage nor previously painted strokes.

For example, start at canonical coverage 128 and paint black at full opacity/flow. Masked Areas starts its working destination at 128; Normal and Multiply both produce coverage 0. Selected Areas starts its working destination at 127; Normal and Multiply both produce working value 0 and therefore coverage 255. Inverting only the black source to white while blending directly into canonical coverage would incorrectly leave Multiply at 128. Test these byte-exact endpoints for strokes and fills, and use the existing blend engine's rounding for partial-opacity cases.

Alternative: store protection and invert on every boundary. Rejected because selection/channel APIs already use coverage and unnecessary conversion risks losing soft values or reversing paint twice.

### 2. Mode state is separate from the live gesture; selection owns coverage

Add document-local Quick Mask mode and grayscale-view state beside `DocumentSession.selection`. Do not occupy `ActiveSession` for the whole mode: a Quick Mask contains many independent brush strokes, fills, and filter previews. Those gestures continue to use the existing single-session admission and cancellation lifecycle.

Keep `session.selection` as the authoritative working coverage. While Quick Mask is active, `None` reads as all-selected for painting and presentation, but remains `None` until an edit commits. `Some(zero_mask)` remains a fully protected explicit empty selection. Entry and an untouched exit therefore neither allocate a full-canvas mask nor lose the absence-of-selection sentinel. A first edit of `None` begins with a white mask and commits `Some(edited_coverage)`.

Mask painting uses a temporary one-channel painting document, with no selection clip. Masked Areas can share the canonical plane until it is written; Selected Areas converts only touched rows/tiles into its working convention, or computes the equivalent inverse-space blend directly for touched coverage. Do not allocate or invert another full-canvas plane on each stroke or dab. Preview crops use the same conversion within their bounded regions; whole-mask filters retain their existing allocation/admission budget. Preview and writeback cross the polarity boundary exactly once and update only canonical coverage and its revision. Never append a fake layer or temporary alpha to the real document. Quick Mask target resolution precedes the existing layer-mask branch, so an active layer mask cannot intercept the stroke. Preserve its edit-target state for ordinary editing after exit; target invalidation still follows existing layer lifecycle rules.

Alternative: a second persistent mask plus independent undo snapshots. Rejected because it duplicates selection state and requires reconciling two histories on exit.

### 3. Use the current selection history across the mode boundary

Before a stroke, fill, or filter, retain the existing selection baseline, including `None`. On commit, publish the resulting selection through the normal selection-edit transaction and record one state if coverage changed. Undo/redo restores that coverage through the current history mechanism. In Quick Mask, restored `None` displays an all-clear default mask; outside it, it remains no active selection.

Entry and exit change only mode/presentation state. Exit exposes the current coverage to ordinary selection consumers and invalidates the contour cache; it does not threshold the mask or append a redundant state. An edit's undo after exit consequently restores the pre-edit selection. Cancel restores the pre-gesture coverage and clears preview presentation without recording. A request to leave during a pointer stroke first uses the existing gesture finalization; options/polarity changes are deferred until that stroke finishes or is cancelled. A modal filter retains ownership until accepted or cancelled.

This uses the tool document's per-edit model rather than the selection document's optional whole-session checkpoint. No change to the ordinary history depth bound or serialization is needed.

### 4. Reuse concrete grayscale adapters and fail closed

Implement explicit Quick Mask routing before layer-pixel and layer-mask routes in paint, fill, and filter entry points. Audit the separate content-move and duplication entry points as well: `move_selection_content`, Move/Ctrl-drag, Alt duplication, and their preview setup/update paths can otherwise edit pixels or a layer mask independently of painting dispatch. Refuse those content-moving operations while Quick Mask is active, both in UI admission and at backend boundaries before any pixel/layer-mask branch or preview allocation. Do not introduce mask-content geometry operations. If entry into Quick Mask encounters an existing content-move preview, cancel it through the normal lifecycle and clear its canvas offset/duplicate presentation before showing the mask. A refused attempt must leave canonical optional selection/coverage, layer masks, layer/composite pixels, layer target and history unchanged and leave no transient move preview behind. Share the existing one-channel mask painting preparation, dab engine, and filter math where inputs are identical; extract only helpers with both real callers. Do not introduce a general target framework or trait.

Brush/Pencil use the configured opacity, flow, spacing, and grayscale foreground. Eraser paints the grayscale background rather than transparent image pixels. Paint Bucket and Gradient operate on the mask image and never use it as their own clipping selection. Filters use the grayscale-mask path, including modal preview/apply/cancel; operations requiring unsupported color or source semantics refuse before reaching layer edits. Healing, Pattern Stamp, and Erase To History retain their existing mask-target refusals.

The backend's paint admission checks the selected-layer target, and the tool cursor uses the same admission result. An empty target produces a blocked cursor and a refused stroke with no edit/history. A selected layer is an admission requirement, not the destination for Quick Mask pixels. Pixel locks must not accidentally turn a coverage edit into a layer-pixel operation.

### 5. Canvas presentation is a separate source

Generate overlay/grayscale canvas crops from the coverage plane at the same selected pyramid level and document transform as the current composite. Extend the established canvas presentation path rather than replacing the document composite or tinting a full-resolution `image()` accessor. The Qt boundary consumes rendered image crops; it does not copy the raw mask into widgets on every repaint.

Use dirty stroke regions to refresh coverage display, and bounded pyramid crops for pan/zoom and thumbnails. Entering/leaving a selection-backed mask shares its `Plane`; the no-selection baseline stays implicit until painting needs a plane. Avoid full-canvas materialization on every dab, pan, or panel refresh. CPU presentation is the acceptance path and remains available with GPU compute disabled; no GPU-only dependency is introduced. Hide ordinary selection ants while Quick Mask is active.

Alternative: bake red into `doc.composite` or reuse a layer-mask composited preview. Rejected because both contaminate image consumers or make selection protection affect layer visibility.

### 6. Project one mode into the shell

- Route `Q` and a plain toolbar click through one registered command. The button reads the active document's state. Keyboard ownership excludes text editors and owning modal dialogs. Double-click/Alt-click must be recognized before their constituent clicks can toggle the mode; do not rely on the button's independent checkable state.
- Store accepted polarity/color/opacity in `SessionState`; missing fields load Masked Areas/red/50%. Validate opacity and enum values on read. Cancel leaves store and preview unchanged. Give Quick Mask its own black/white paint pair using the existing ColorState projection mechanisms without overwriting the remembered image or layer-mask foreground/background pairs. Preserve edits to the Quick Mask pair during document activation changes while the mode remains active. On exit, resolve the surviving edit target: restore its remembered layer-mask pair if a raster layer mask is still targeted, otherwise restore the remembered image pair. On document activation, project the new document's Quick Mask pair when active, otherwise the pair belonging to its current edit target. Replacement or invalidation of a mask target uses the existing image-target fallback. Keep these paint-pair transitions out of document history and display preferences.
- Give the Channels temporary row an explicit kind, a bounded coverage thumbnail and no durable alpha index. Selecting it chooses grayscale view. Disable its durable-channel operations. Selecting an ordinary channel first exits through the same Quick Mask command, then performs the existing row action.
- Project the mode and active layer path into the Layers row delegate. A shared theme token for the red active highlight overrides only the active row's usual highlight, including across focus/repaint. In a multi-selection it follows the existing active anchor; other selected rows keep their normal selection styling. This never writes the layer's color label. No active layer means no red row. Add color tokens through the theme and regenerate the existing styleguide if required by that token change.

Each document retains its own mode/view state on activation changes; global options and ColorState are projected from the newly active document. A newly opened document starts in Standard mode. Document replacement clears mode, temporary row and display caches. Because coverage is the canonical selection rather than a second geometry-bearing mask, ordinary selection geometry changes cannot leave a differently sized shadow mask behind; refresh Quick Mask presentation whenever that selection is changed or cleared.

### 7. Persistence stays on existing paths

The real `Document`, channels, composite, and codec schema never receive Quick Mask state. Saving during the mode naturally omits it. The codec still serializes only durable alpha channels. After exit, Save Selection uses the existing command and its validation; the PSD round-trip check compares exact channel bytes and image pixels, not a new Quick Mask PSD resource. Use runtime-generated fixtures without baseline churn.

### 8. Test through public behavior

Use a dedicated offscreen `tst_quick_mask` suite with independent fixtures, actual canvas gestures/cursor reads, canvas samples, toolbar interactions, channel thumbnails, and Layers delegate rendering. Add Rust std tests for coverage, admission, history, cancellation, and save isolation. Paint an initially selected region and its exterior to catch self-clipping. Compare layer/channel/composite bytes before and after edits and saves. Include default and reversed polarity with Normal/Multiply strokes and fills on coverage 128, refused content moves/duplicates and preview cleanup with pixel and layer-mask targets, custom image/mask color-pair restoration across document switches, 0/100% opacity, no-selection versus explicit empty masks, soft values around 127/128, 1-pixel canvases, multiple documents, CPU display and 2x rendering.

`test-plan.md` maps every scenario to proposed test names. Those names are prospective until apply writes the tests. Planning does not add placeholder tests just to satisfy the implementation guard. A separate manual row records lawful CS6 comparison for unverified historical details; automated acceptance does not claim that oracle exists.

## Risks / Trade-offs

- **Coverage can clip itself or hit a layer mask** -> explicit target precedence, no clipping mask in the temporary paint document, and tests outside the entry selection and with a layer mask active.
- **Content-move paths bypass painting dispatch** -> refuse their backend and UI entry points and previews; assert unchanged coverage, layer masks, pixels and history, including cleanup when entering from an existing preview.
- **Source-only polarity inversion breaks non-Normal blends** -> edit the destination in the captured polarity's working convention and convert preview/commit output back once; verify Normal and Multiply against the coverage-128 arithmetic case.
- **Quick Mask swatches overwrite remembered colors** -> retain a separate Quick Mask pair and restore by the surviving document edit target; test custom layer-mask and image pairs across exit and document activation.
- **Mode state can occupy the only gesture slot** -> store it separately and keep each stroke/filter in the normal slot.
- **UI can toggle twice on double-click** -> gesture-aware button handling tested in both modes, including dialog cancel.
- **Selection restoration can lose the `None` sentinel** -> keep it canonical until an edit commits and test undo across the exit boundary.
- **Layer highlight can become durable label data** -> theme/delegate state only; compare labels and history before/after.
- **Large masks can cause flat-buffer copies** -> retain shared planes, bound visible crops and thumbnails, and add an allocation/materialization regression at the selection/presentation boundary.
- **CS6 history granularity, native temporary-channel depth and exact filter availability lack independent observations** -> document the chosen 8-bit/project-history/grayscale-tool ceilings; do not claim native-depth or full CS6 tool parity. Future captures can refine separate behavior changes.
- **Existing long-form docs still contain an all-masked no-selection proposal** -> the user-confirmed capability scenario is explicit; any later docs reconciliation needs its own authorized docs change.

## Migration Plan

This is an additive runtime feature with optional preference fields and no PSD migration. Implement tests and all source changes before the single initial build; compile with at most 80% of available CPUs. Run the focused suites and required gates after that build. Update `ROADMAP.md`, include the manual comparison row as a PR reviewer checklist, validate and archive the change before the shipping PR completes. Rolling back the implementation leaves saved image files unchanged and older preference readers ignore the new optional fields.
