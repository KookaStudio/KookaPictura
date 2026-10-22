# App UI issues investigation

Working note (`docs/dev/`). Investigation only: no code was changed. Nineteen
reported issues were traced by seven parallel code investigations plus web
research on Krita's brush architecture for the performance item. Line
references are current at commit `7ee4805`.

Each issue lists the confirmed root cause with `file:line`, ranked candidate
solutions, and the specs/tests that would move. Sizes are rough: **S** (one or
two localized edits), **M** (a focused change across a few files), **L** (model
or cross-crate change).

## Summary

| # | Issue | Root cause | Size | Spec delta |
|---|---|---|---|---|
| 1 | Panel width/mode lost on restart | quit path never saves; width not in schema; per-column width missing | S-M | `workspace-persistence`, `panel-column` |
| 2 | 512x512 document on startup | scratch document created unconditionally in `main.cpp` | S | `application-shell`, `document-lifecycle` |
| 3 | Opened file tab says `Untitled-n` | raster Open drops the path; no mode/depth accessor | S-M | `document-tabs`, `image-import` |
| 4 | Opened raster is a named, unlocked, alpha layer | `Document::from_rgba` names by stem, no locks, always adds alpha | M | `image-import`, `layer-management` |
| 5 | Double-click anywhere renames | default `DoubleClicked` edit trigger | S-M | `layers-panel` |
| 6 | Lock position does nothing | Move tool / translate never check `POSITION` | M | `layer-management` (or new `layer-locks`) |
| 7 | Arrow-key nudge missing | no arrow-key handler; move path is topmost-layer only | M | `canvas-tools`, `edit-history` |
| 8 | Brush slow at 4000x4000 | cumulative stroke dirty rect; CPU region does a full composite | M | `paint-engine` |
| 9 | Lock pixels does nothing | `PIXELS` round-tripped but never consulted | M | `layer-locks`, `brush-tools`, `filter-application` |
| 10 | Layer drag only extends selection | model flags lack `ItemIsDragEnabled` | S | `layers-panel` (already specified) |
| 11 | Color label does not tint eye column | delegate paints the swatch after the name only | S | `layers-panel` |
| 12 | Ctrl+G groups only the current row | accelerator handler bypasses the selection-aware op | S | `layers-panel` (already specified) |
| 13 | Numeric labels are not scrubby | only `PercentField` has a scrub label; every other control uses a plain label | S-M | new `numeric-fields` |
| 14 | Numeric fields have no slider popup | only `PercentField` has one; 14 bare spin boxes elsewhere | M | new `numeric-fields` |
| 15 | Slider click does not drag | Fusion pages on left-click; `QSlider` tracks only from the handle | S-M | `navigator-panel`, `color-swatches-panel` |
| 16 | Popup slider ignores Left/Right | popup never focuses the slider; no key handler | S | `numeric-fields`, `layers-panel` |
| 17 | Brush size has no circle cursor | static 24-px SVG cursor, never scaled; size changes emit nothing | M | `brush-tools`, `tool-framework`, `svg-cursors` |
| 18 | Canvas can be panned fully off-screen | `offset_` is never clamped at any mutation point | S-M | `canvas-tools` |
| 19 | No workspace scrollbars | canvas is a plain tab page, no scroll area | M | new `canvas-scrollbars`, `document-tabs` |

Issues 10, 12 and 15 are code-vs-existing-spec mismatches. Issues 3, 4, 7, 8
and 17 contradict current specs or unspecified behavior and need MODIFIED
requirements. Issues 13, 14, 16, 18 and 19 are mostly new capabilities. Note
that `PercentField` already implements 13, 14 and 15 for one control, so those
are a generalization problem rather than new invention.

---

## 1. Panel width and mode do not survive a restart

**Root cause.** The workspace store is `$XDG_STATE_HOME/kooka-pictura/state.json`
(schema v6, `cpp/session.h:33`, `cpp/session.cpp:65`). Several things go wrong:

- **The standard quit path never saves.** The only unconditional save is in
  `PicturaMainWindow::closeEvent` (`cpp/frame.cpp:782`). `File > Exit`/`Ctrl+Q`
  runs `qApp->quit()` (`cpp/frame_menus.cpp:126-133`), which leaves the event
  loop without calling `closeEvent`, so `saveSession()` never runs. No
  `aboutToQuit` connection exists.
- **Resizing a column has no save trigger.** Column width is only captured by
  the quit-time save. There is no `splitterMoved` hook; the only
  `PanelColumn::stateChanged -> saveSession` wiring (`cpp/frame_columns.cpp:5`)
  does not fire on splitter drags.
- **Only the primary column width is stored.** `saveSession` writes
  `railWidth` from `panelColumn_` only (`cpp/frame_session.cpp:67-68`), and the
  v6 `panelColumns` entries carry `side`/`order`/`groups` but no width. Every
  non-primary column width is lost even on a clean window close.
- **Saved in iconic mode loses the normal width.** Restore skips
  `setPreferredWidth` when iconic (`cpp/frame_session.cpp:196`), and
  `setRailMode(true)` only records the pre-iconic width when the widget already
  has one (`cpp/panel_column_iconic.cpp:57`), which is 0 during construction.
- **Latent ordering fragility.** `applyPanelSession` runs before the
  constructor resize and `show`; the width is deferred in `pendingWidth_` and
  applied from a single `showEvent` (`cpp/panel_column.cpp:430`). If the
  splitter is not laid out then, the width is dropped.

**Candidates.**
1. Make quit always save: route `FileExit` through `close()` or connect
   `QApplication::aboutToQuit` (`cpp/frame_menus.cpp`, `cpp/main.cpp`). Small,
   covers every quit path.
2. Persist width on splitter drag, debounced via a single-shot timer
   (`cpp/frame.cpp` / `cpp/frame_columns.cpp`).
3. Add `width` to each column entry and apply it; v6->v7 migration
   (`cpp/session.*`, `cpp/frame_session.cpp`). The only way multi-column widths
   are correct.
4. Seed `normalWidthBeforeIconic_` from `railWidth` (`cpp/panel_column.*`).
5. Re-apply the width after first layout with a zero-delay timer.

**Specs/tests.** `openspec/specs/document/workspace-persistence/spec.md` and
`openspec/specs/ui/panel-column/spec.md` only assert the stored `railWidth`
round-trips (self-test checks 128, 151); add a real apply-and-measure restart
requirement. `docs/10-workflow-io/workspace-management.md:153-156` already
models `panel_widths` but it is unimplemented.

---

## 2. Startup always creates a 512x512 document

**Root cause.** `cpp/main.cpp:157` unconditionally builds a scratch white
document for any launch that did not load a CLI path:

```cpp
if (!codecLoaded) {
    frame.newDocument(QStringLiteral("Untitled"), 512, 512, ..., "white");
    ...
}
```

It is not gated on self-test, although the self-test depends on it
(`null_image`, `fresh_white`, and `gpu_blank` at `cpp/selftest.cpp:81-108`).
The empty-workspace state itself already works: `activeDocumentIndex()` returns
-1 with no tabs (`cpp/frame.cpp:197-203`), and command enablement is
`documentCount() > 0`. The `application-shell` spec already has a "no document
open" scenario.

**Candidates.**
1. Gate the scratch document on self-test (`cpp/main.cpp`). Normal launch opens
   nothing; `--headless`/`--self-test` keep the document their checks need.
2. Optional empty-state placeholder in the tab area (cosmetic).
3. Opt-in session restore: add `openDocuments`/`restoreOpenDocuments` to
   `SessionState` (`cpp/session.*`), a checkbox in `preferences_dialog.*`
   mirroring `Use Shift Key For Tool Switch`, and open the saved paths on
   startup when enabled (`cpp/frame.cpp` / `cpp/frame_session.cpp`). Only saved
   paths can be restored; untitled dirty buffers were never persisted.

**Specs/tests.** `application-shell` "no document" and `document-lifecycle`
recent-files. The self-test checks that assume the startup document must stay
valid under `--self-test`.

---

## 3. Opened file shows as `Untitled-n` instead of `name (RGB/8)`

**Root cause.** Two defects.

- The raster Open path deliberately drops the path. `openImagePath` imports into
  an untitled PSD document and calls `addDocument(view, QString())`
  (`cpp/frame.cpp:361-372`), so `documentName` falls back to the untitled
  counter (`cpp/frame.cpp:236-246`). The Rust side keeps only the file stem, and
  even that goes to the layer name (`src/cxxqt_object/impl_core.rs:46-57`).
- No mode/depth is exposed. `updateTabTitle` appends only the dirty marker
  (`cpp/frame.cpp:712-723`); the bridge declares only `document_width`/`document_height`
  (`src/cxxqt_object.rs:124-129`). `Document.mode`/`depth` exist in the engine
  (`crates/pictura-core/src/lib.rs:75-79`) but there is no label helper.

**Candidates.**
1. Add a display name to `DocEntry` distinct from the save path, set it in
   `openImagePath`/`openAsSmartObjectPath`, and prefer it in `documentName`
   (`cpp/frame.h`, `cpp/frame.cpp`). Keeps Save semantics intact.
2. Add bridge accessors `document_mode()`/`document_depth_bits()` plus mode
   label helpers, and format the title as `base + " (" + mode + "/" + bits + ")"`.
3. Keep `Untitled-N` when there is no file name; decide whether the dirty `*`
   goes before or after the suffix.

**Specs/tests.** `openspec/specs/document/document-tabs/spec.md:32-47` and
`openspec/specs/interop/image-import/spec.md:156-181`. No existing test asserts tab
text; `lpr_image_import` (self-test check 290) only checks
`file_path().isEmpty()`.

---

## 4. Opened raster should be a locked opaque Background layer

**Root cause.** `Document::from_rgba` always creates one layer named after the
file stem, with default (unlocked, non-background) attributes and a `-1` alpha
channel:

```rust
// crates/pictura-core/src/lib.rs:131-186
doc.layers.push(Layer {
    name: name.to_string(),
    ...
    ..Default::default()          // background: false, lock: default
});
```

The import path is `impl_core.rs:46-57`. Qt always decodes to
`Format_RGBA8888` (`cpp/decode_image.cpp:14`), so even a JPEG arrives with a
fully opaque alpha plane. Dropping the alpha plane is safe: the compositor
treats a missing `-1` channel as opaque (`crates/pictura-render/src/composite.rs:139-154`).

The Background concept already exists: `Layer.background`
(`crates/pictura-core/src/lib.rs:543-547`), the codec's derive/write
(`read.rs:416-424`, `write.rs:76-84`), the engine ops `background_from_layer`
and `layer_from_background` (`create.rs:494-531`), and the menu command
`Layer > New > Layer from Background` (`cpp/frame_menus.cpp:407-417`). What is
missing is the import-side setup and the panel double-click/lock-click
affordance (currently only the menu entry works).

**Candidates.**
1. Post-process in `open_image` (`impl_core.rs`): rename to `Background`, set
   `background = true`, `lock = LockFlags::all()`, drop the `-1` channel.
   Smallest change; leaves `from_rgba` and the Place path alone.
2. Change `Document::from_rgba` itself. Higher blast radius; the
   `image-import` spec currently requires stem naming and preserved alpha.
3. Add a small engine helper that combines rename plus `background_from_layer`.
4. Wire the conversion affordance: hit-test `delegate_->lockRect(...)` in the
   viewport event filter (`cpp/layers_panel.cpp:601`) or connect
   `tree_->doubleClicked`, then call `layer_from_background`. Overlaps issue 5.

**Decision needed.** An image with real transparency (PNG) should arguably stay
a regular layer with alpha. Confirm the rule is "opaque image becomes
Background".

**Specs/tests.** `openspec/specs/interop/image-import/spec.md:49-80,156-181`
contradict the requested behavior. `openspec/specs/compositing/layer-management/spec.md:183-220`
defines the Background semantics. Extend `lpr_image_import` (check 290).

---

## 5. Double-click renames anywhere; should rename only on the name, else Layer Style

**Root cause.** The tree keeps the `QAbstractItemView` default edit triggers,
`DoubleClicked | EditKeyPressed`; no `setEditTriggers` call exists anywhere.
The model marks every row editable (`cpp/layers_panel_internal.h:280`), and Qt's
`mouseDoubleClickEvent` calls `edit()` unconditionally after emitting
`doubleClicked`. The delegate computes the name rectangle as local variables in
`paint()` (`cpp/layers_panel_internal.h:636-647`) and exposes no `nameRect`
hit-test (only `eyeRect`, `lockRect`, `chevronRect`).

There is no Layer Style dialog to open yet: the fx button is disabled with a
"not implemented yet" tooltip (`cpp/layers_panel.cpp:238-240`), Blending
Options is disabled in the context menu, and the menu-bar entries are inert.

**Candidates.**
1. `setEditTriggers(NoEditTriggers)` plus a `MouseButtonDblClick` branch in the
   existing viewport event filter; add `nameRect` to the delegate; rename inside
   it, otherwise call a placeholder `openLayerStyle(path)` (a documented no-op
   until the dialog exists). The only reliable way to suppress Qt's built-in
   edit.
2. Override `mouseDoubleClickEvent` in `LayersTreeView` instead of the filter.
3. Gate `LayersModel::setData`/`flags` behind a rename flag. Rejected: fragile
   shared state.

**Specs/tests.** `openspec/specs/ui/layers-panel/spec.md` "layer property editing"
and the m39 rename check.

---

## 6. Lock position does not work

**Root cause.** `LockFlags::POSITION` is enforced only in the free-transform
path (`impl_transform.rs:302-314`, `layer_ops/transform.rs:278-284`). The Move
tool does not route through either: press starts a preview
(`cpp/tools.cpp:609-628`), release calls `commit_move`
(`cpp/tools.cpp:849-857`), which ends in `translate_layer_rect`. None of the
three `translate_layer*` functions check a lock
(`crates/pictura-render/src/document_ops/crop.rs:51-104`); they just move the
topmost pixel layer. Content move is likewise unguarded. Structural panel
reordering is correctly not blocked by position lock.

**Candidates.**
1. A shared `pictura-render` predicate (not group/adjustment/background, and not
   `POSITION`/`is_all()`) called from the three `translate_layer*` functions and
   the preview builder, plus content move. Covers drag, content move, and the
   future nudge.
2. Guard at the C++ tool entry with a new bridge query. Smaller diff but leaves
   bridge callers unguarded.
3. Do not filter in `topmost_pixel_layer*` itself: that selector is shared with
   `apply_filter`, and position lock must not block pixel edits.

Note: the Move tool always targets the topmost pixel layer, not the panel
selection (`impl_transform.rs:744-746`); that is a separate gap.

**Specs/tests.** Add position-lock enforcement for Move-tool drag, content move,
and nudge to `layer-management` or a new `layer-locks` capability. Rust tests
in `crop.rs` and `cxxqt_object/tests.rs:478`; new self-test check.

---

## 7. Arrow-key nudge of the selected layer(s), coalesced

**Root cause.** Arrow keys are unimplemented: `rg` for `Key_Left` etc. in
`crates/pictura-app/cpp` finds nothing. `ImageView::keyPressEvent` forwards and
`PicturaMainWindow::keyPressEvent` (`cpp/frame.cpp:786-814`) does not handle
arrows.

The move machinery can be reused: drag accumulates a delta without touching the
document (`cpp/tools.cpp:772-779`) and commits once on release via
`commit_move` (`impl_transform.rs:834-868`), which pushes one `record_move`
history state. But the move path is hard-wired to the topmost pixel layer
(`helpers.rs:593-597`), and multi-selection lives only in the panel
(`LayersPanel::selectedPaths()`, `cpp/layers_panel.cpp:553-567`), not in the
view. There is no multi-layer translate in the renderer. No idle coalescing
timer exists for moves.

**Candidates.**
1. Minimal single-layer nudge reusing the move preview and a single-shot timer
   (~500 ms to 1 s) that commits one history state per burst.
2. Multi-layer nudge: add a path-based `translate_layers_rect`, a bridge
   `nudge_layers(paths, dx, dy)`, and a deferred record. Note `record_move`
   deliberately does not bump `content_revision`; a multi-layer move must
   invalidate the single-layer preview cache.
3. Zoom-scaled step and Shift multiplier. **This is a spec conflict to
   escalate:** `docs/dev/canvas-view-spec.md:141` specifies a fixed 1 px step
   (10 px with Shift), while the issue asks for zoom scaling, and
   `docs/02-ui-ux/keyboard-shortcuts.md:389-392` assigns plain arrows to
   selection and `Ctrl+arrow` to layer movement in CS6.

**Specs/tests.** `openspec/specs/tools/canvas-tools/spec.md:6-17`,
`openspec/specs/document/edit-history/spec.md`, and the stale
`docs/dev/canvas-view-spec.md:138-169`.

---

## 8. Brush is very slow on large documents (4000x4000)

### What the code does today

The path is CPU-only (GPU painting is deferred). Per mouse move:
`cpp/tools.cpp:815-819` calls `paint_dab`; the bridge samples the dab and, on
change, refreshes the stroke's dirty rect (`impl_paint.rs:63-86`).

### Where the time goes (root causes, ranked)

- **RC1: the refresh rect is cumulative, so each dab recomposites the whole
  stroke so far.** `Stroke::dirty()` only grows (`crates/pictura-paint/src/stroke.rs:132-172`);
  nothing clears it until `finish`. This is O(L^2) per stroke and directly
  violates the repo's own budget ("only the dab's bbox", <=16 ms/dab,
  `docs/dev/canvas-view-spec.md:206,221`).
- **RC2: the CPU region fallback composites the whole document.**
  `composite_cpu_region` calls `composite_rgba(doc)` then slices
  (`crates/pictura-render/src/gpu/mod.rs:185-205`). On a 4000^2 document that is
  about a second per mouse move when the GPU path is unavailable.
- **RC3: `Stroke::begin` deep-clones the document twice**
  (`crates/pictura-paint/src/stroke.rs:51-68`), plus two layer-sized planes.
- **RC4: per-pixel constant cost.** The tip profile recomputes `sin_cos` per
  pixel (`crates/pictura-paint/src/tip.rs:11-14`) and pixel reads/writes
  linearly scan the channel vector (`stroke.rs:189-319`).

Measured on this machine: full 4000^2 GPU composite about 190 ms, CPU about
1064 ms; GPU region refreshes 512^2 about 5 ms, 1024^2 about 11 ms.

### How Krita solves it (web research)

Krita is a CPU-first painter on large canvases; its answer is architecture, not
a faster inner loop alone.

- **Tiled pixel storage with copy-on-write and swap.** `KisTiledDataManager`
  divides devices into 64x64/128x128 tiles, shares tiles between layers and
  undo states until modified, compresses inactive tiles, and swaps them to disk
  when memory is tight (`KisTiledDataManager`, swap subsystem). Large documents
  never need one contiguous allocation, and undo snapshots are cheap.
- **Indirect painting device.** A freehand stroke paints dabs into a temporary
  paint device and merges it into the layer in the stroke's final job. This
  isolates the layer from per-dab churn and makes cancellation exact
  (Krita "Strokes queue": init job starts a transaction, dab jobs paint, finish
  job merges and commits undo).
- **Dirty-rect updates, not full recomposes.** Updates are broken into region
  jobs and run by a scheduler across threads (`KisUpdateScheduler`,
  `KisUpdaterContext`, `KisProjectionLeaf`). Krita's own optimization notes
  list "projection recomposition doesn't take the visible area into account" as
  a known hot spot, i.e. restricting recomposition to the dirty/visible region
  is the fix in this class of engine.
- **Transaction-like undo.** Stroke undo is added after execution through a
  post-execution undo adapter, so the stroke never pays for a full-document
  snapshot per dab; undo/redo of a stroke is scheduled with barriers.
- **Iterator and allocation hygiene.** Reuse tile iterators across rows instead
  of creating one per row, avoid per-iteration temporaries, and vectorize the
  brush path (the optimization page calls random access into the tile manager
  the remaining 10% of slow cases).

### Candidate solutions for this codebase

1. **Incremental per-dab dirty (RC1, small, high GPU impact):** add
   `take_dirty()` and use it in `paint_dab`; keep the cumulative `dirty()` for
   `finish`. Region shrinks to the current dab.
2. **A real region compositor for the CPU fallback (RC2, medium, high CPU
   impact):** remove the `composite_rgba + slice` path; needs byte-parity tests
   against `composite_rgba`. This is the deferred `ponytail:` ceiling.
3. **Coalesce refreshes to the frame rate (S3):** accumulate the dirty union and
   flush on a short timer, mirroring `panelRefreshTimer_` (`cpp/frame.cpp:93-98`).
   Bounds event rate; pair with 1/2.
4. **Patch the cached composite with the painted layer (S4):** source-over the
   painted region into `doc.composite` instead of re-running the stack; the
   region patch machinery already exists (`cpp/helpers_composite.rs:82-113`).
   Masks, groups, and clipping need care.
5. **One clone instead of two at stroke start (RC3, small):** `mem::take` or
   copy-on-write for the stroke base.
6. GPU painting or resident per-layer GPU buffers stay deferred.

The minimal set for the reported symptom is 1 + 2, optionally 3.

**Specs/tests.** `openspec/specs/tools/paint-engine/spec.md` has no latency/region
requirement; add one citing the canvas-view budget. Extend the ignored
profiling home (`src/cxxqt_object/tests_impl.rs`) with a brush-dab profile and
add a `Stroke` unit test for incremental versus cumulative dirty.

---

## 9. Lock image pixels does nothing

**Root cause.** `LockFlags::PIXELS` round-trips through the codec and the panel
but is never consulted. `rg` for `contains(LockFlags` finds only `POSITION` and
`NESTING`/`is_all()`. Mutation entry points with no lock check: brush
`Stroke::begin` (`crates/pictura-paint/src/stroke.rs:39-69`), filters
(`impl_filters.rs:36-59`), and content move (`impl_selection.rs:124-163`).
`TRANSPARENCY` is likewise unenforced. There is no block cursor asset and no
refusal dialog on the canvas path; `Qt::ForbiddenCursor` is a builtin fallback.

**Candidates.**
1. One shared predicate `layer_pixel_locked(layer)` in the engine, checked at
   each mutation entry (paint, filters, content move, fill). Smallest correct
   core change; needs a `PaintError::Locked` variant.
2. Cursor: extend `ToolController::refreshCursor` to consult a bridge query and
   set `Qt::ForbiddenCursor` for pixel-editing tools. If a custom asset is
   wanted, add it to `assets/cursors/` and the explicit `assets/pictura.qrc`.
3. Refusal dialog on a `false` return from `begin_paint`/`apply_filter`,
   following the `QMessageBox::warning` pattern in `cpp/frame_menus.cpp:708`.

**Specs/tests.** `openspec/specs/ui/layers-panel/spec.md:41-60` only reports or
edits lock state; enforcement belongs in `layer-locks`/`brush-tools`/
`filter-application`. Rust tests in `stroke.rs`, `move_content.rs`,
`impl_filters`; a new self-test check plus a cursor check.

---

## 10. Dragging a layer only extends the selection

**Root cause.** The whole drag-and-drop pipeline is implemented but dead: the
model flags lack the drag/drop bits, so Qt never starts a drag and falls back
to rubber-band selection.

```cpp
// cpp/layers_panel_internal.h:285
return Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsEditable;
```

`LayersTreeView` overrides `startDrag`/`dragMoveEvent`/`dropEvent`, the panel
wires the path provider and validator, the bridge exposes `can_move_layer_to`
and `move_layer_to` with refusal guards, and drop-on-strip buttons are handled
in `LayersPanel::eventFilter` (`cpp/layers_panel.cpp:571-599`). The self-test
only checks `dragEnabled()`/`acceptDrops()` and calls the bridge directly, so
the dead pipeline was never exercised. The spec already requires the opposite
of the observed behavior (`openspec/specs/ui/layers-panel/spec.md:648-663`).

**Candidate.** Add `Qt::ItemIsDragEnabled | Qt::ItemIsDropEnabled` to
`LayersModel::flags()`. One line; activates the existing, spec-tested pipeline.
Add a regression test that synthesizes a press/move/release and asserts no
selection extension plus one reorder history step.

---

## 11. Color label should tint the eye column

**Root cause.** The delegate paints the eye glyph without a background and
paints the label only as a small swatch after the name
(`cpp/layers_panel_internal.h:558-562,649-658`). The label color function
`layerLabelColor` is file-local in `layers_panel.cpp:70-90`, so the delegate
cannot obtain the color for a fill.

**Candidates.**
1. Export `layerLabelColor` to `layers_panel_internal.h`, and in
   `LayerRowDelegate::paint` fill the eye gutter before the glyph:
   `QRect(rect.left(), rect.top(), kEyeColumn, rect.height())` with the label
   color. Purely visual; matches Photoshop's left gutter.
2. Alpha-blend the fill so the eye and selection highlight stay legible.
3. Cache a per-label pixmap. YAGNI.

**Specs/tests.** `openspec/specs/ui/layers-panel/spec.md` "layer row badges and
delegate". Add a paint-and-sample check using the existing geometry test hooks.

---

## 12. Ctrl+G should group the selected layers

**Root cause.** Ctrl+G is bound (`cpp/command_tree.cpp:416-417`) but the
handler groups only the current row through the single-layer op:

```cpp
// cpp/frame_menus.cpp:309-311
view->group_layer(layersPanel_ ? layersPanel_->currentLayer() : -1);
```

`currentLayer()` returns -1 for nested paths, and a multi-selection groups only
one layer. The selection-aware path already exists and is correct:
`LayersPanel::groupSelection` calls `view->group_layers(paths)`
(`cpp/layers_panel_actions.cpp:130-146`), the engine `group_paths` performs the
required refusals (`layer_ops/properties.rs:255-271`), and the bridge
`group_layers` recomposites and records one undo step (`impl_layers.rs:626-642`).
The spec already requires the menu to act on the selected layers
(`openspec/specs/ui/layers-panel/spec.md:184-223`).

**Candidate.** Rewrite the `LayerGroupLayers`/`LayerUngroupLayers` handlers to
mirror `groupSelection`/`ungroupSelection` using `selectedPaths()` and the
`group_layers`/`ungroup_layers` bridge methods. Small, localized; undo and
refusals already work. Add a menu-level self-test check.

---

## 13. Drag a numeric label to scrub its value

**Root cause.** One scrub implementation exists and it is instance-local.
`PercentField` (`cpp/panels/percent_field.cpp:84-102,217-244`) makes the label,
the edit, and the `%` suffix scrub handles (`Qt::SizeHorCursor`, a 3-px dead
zone, 1 unit per px). Every other numeric control uses a plain `QLabel` or a
`QFormLayout` row label: options bar (tolerance, feather, brush size/hardness/
opacity/flow, fixed size/ratio) at `cpp/options_bar.cpp:92-332`, new-document
and new-layer dialogs (`new_document_dialog.cpp:26-31`, `layer_new_dialog.cpp:54`),
preferences brightness (`preferences_dialog.cpp:61`), and the color panel axes
(`color_panel.cpp:124-141`). There is no modifier scaling even in
`PercentField`; the `ponytail:` comment at `percent_field.cpp:241` names that
ceiling.

**Candidates.**
1. A reusable `ScrubLabel` (QLabel subclass or event-filter helper) applied to
   the plain-label sites, with sensitivity 1/px and Shift/Ctrl modifiers;
   refactor `PercentField` onto it.
2. Fold the label into a `NumericField` (label + spin box + popup), which
   resolves issues 13 and 14 together.
3. Per-label event filters. Rejected duplication.

**Specs/tests.** No `options-bar` capability exists; the shared contract
belongs in a new `numeric-fields` capability, with call-site amendments in
`tool-framework`, `brush-tools`, `selection-tools`, `shape-selection-tools`,
`new-document`/`layers-panel` dialogs, and `panel-*`. No scrub test exists
today.

## 14. Slider popup on numeric fields

**Root cause.** `PercentField` is the only field with a slider popup: a
`QToolButton` opens a `JumpSlider` inside `QWidget(this, Qt::Popup)`
(`cpp/panels/percent_field.cpp:110-125,200-206`). It is not generic (hardcoded
0-100, `%` suffix, no decimals). Every other numeric control is a bare
`QSpinBox`/`QDoubleSpinBox`: options bar (5 + 4 in the paint rows,
`options_bar.cpp:93,133,202,227,231,245,248,315`), new-document
width/height, new-layer opacity, preferences brightness. Navigator and color
panel are slider-first with no text field.

**Candidates.**
1. Generalize `PercentField` into a configurable `NumericField` (min/max/step/
   decimals/suffix/int-vs-double/optional popup) and re-express `PercentField`
   as a thin configuration, then migrate the 14 spin boxes. Mechanical, about
   six translation units plus `CMakeLists.txt` (explicit file list, no globbing).
2. A `NumericSpinBox` subclass that only adds the popup, leaving layouts.
   Smaller diff, duplicates the popup logic.
3. Leave color/navigator as slider-first controls; a numeric box beside them is
   a later addition.

## 15. Press-drag on a slider should track, not just jump

**Root cause.** The app uses the Fusion style (`cpp/theme.cpp:274`), whose
`QCommonStyle` defaults are `SH_Slider_AbsoluteSetButtons = MiddleButton` and
`SH_Slider_PageSetButtons = LeftButton`. A left press on the groove therefore
page-steps (with auto-repeat) and never enters the tracking branch:
`QSlider::mouseMoveEvent` returns early unless `pressedControl == SC_SliderHandle`.
`PercentField` already worked around this with a private `JumpSlider`
(`percent_field.cpp:21-71`) that forces `setSliderDown(true)` and tracks. The
raw `QSlider`s still affected are the navigator (`navigator_panel.cpp:161`) and
all six color sliders (`color_panel.cpp:126,135`). `HueSpectrum` and the
navigator thumbnail are custom and already track.

**Candidates.**
1. Hoist `JumpSlider` out of the anonymous namespace into a shared header and
   use it for the navigator and color sliders. Behavior is known-good and
   covered by self-test `lpr_slider` (code 218).
2. A global `QProxyStyle` overriding `SH_Slider_AbsoluteSetButtons` to include
   `Qt::LeftButton`. One-point app-wide fix, but changes every slider's feel and
   must preserve the theme setup.
3. Per-file local subclasses. Rejected duplication.

**Specs/tests.** `navigator-panel` and `color-swatches-panel` gain a scenario.

## 16. Popup slider should respond to Left/Right

**Root cause.** `PercentField::showPopup()` shows the popup but never focuses
the slider (`percent_field.cpp:200-206`), and the popup event filter handles
only resize/hide/mouse (`:208-245`), no key events. `QSlider` natively steps on
arrows when focused (StrongFocus policy), but nothing in the `Qt::Popup` window
holds focus, so the keys never reach it.

**Candidates.**
1. `slider_->setFocus(Qt::PopupFocusReason)` after `show()` plus explicit
   `setSingleStep(1)`/`setPageStep(10)`.
2. A `QEvent::KeyPress` branch in the popup filter mapping Left/Right (and
   Home/End/PageUp/PageDown) to `setValue`. Works regardless of focus and is the
   form a generalized `NumericField` should ship.
3. Both.

**Specs/tests.** Extend `lpr_slider` (code 218) with a synthesized arrow key.

## 17. Brush-size circle instead of the plain cursor

**Root cause.** The cursor is chosen by tool id only; brush size never reaches
it. `refreshCursor` sets a static 24-screen-px SVG cursor
(`cpp/tools.cpp:523-527`, assets via `icons.cpp:28-52`), fixed hotspot (2,22)
for Brush/Pencil (`tools.cpp:75-78`). `brushSize_` lives in `ToolController`
(`tools.h:271`, `tools.cpp:367-369`) and is consumed only when a stroke starts
(`tools.cpp:599`); `setBrushSize` emits no signal, so the options-bar spin box
and the `[`/`]` shortcuts do not even resync each other. No brush outline exists:
the only size overlay is the drag-size readout, which paints a text tooltip, not
a circle (`image_view.cpp:604-628`). Mouse tracking and a `mouseMoved` signal
already exist (`image_view.cpp:64,694`), but `handleMoved` returns early when not
dragging (`tools.cpp:761-764`).

**Candidates.**
1. (Recommended) Paint a ring in `ImageView::paintEvent` under the existing
   image transform, driven by a new `setBrushOutline(diameter, imagePos)` and
   set from hover for Brush/Pencil. The circle is in image px, so zoom scales it
   for free; a cosmetic pen keeps the ring 1 screen px and DPR-correct, matching
   the marching-ants technique. Specs already describe an overlay
   (`docs/03-tools/brush-and-pencil.md:89,212`).
2. Rebuild a scaled `QCursor` pixmap on size/zoom. Rejected as primary: OS
   cursor pixmaps cap near 128-256 px, so a large brush or high zoom cannot be a
   cursor.
3. Reuse `setDragSizeHint`. Smallest diff but entangles the marquee text readout
   with brush hover.

**Specs/tests.** Add a `brush-tools` requirement (outline tracks size and zoom),
amend `tool-framework` cursor hints and clarify `svg-cursors`. New self-test
hook `hasBrushOutline`.

## 18. Keep at least one display inch of the canvas visible

**Root cause.** `offset_` is the only pan state and is never clamped. Every
writer is unguarded: `panBy` (`image_view.cpp:140-145`), `setZoom`'s anchor math
(`:748-757`), `centreImage` (`:104-108`), `fitOnScreen` (`:163-170`),
`actualPixels` (`:177-180`), `applyInitialView`/`setImage` (`:79-102`),
`resizeEvent` (`:711-717`), and the navigator proxy
(`navigator_panel.cpp:256-265`). Zoom is clamped but the resulting offset is not.

**Candidate.** One file-local `clampOffset()` using a shared range helper
(`offsetRangeFor(image, zoom, viewport, margin)`), called at every mutation
point so future callers route through it. The margin is expressed in logical
widget pixels because `offset_` and `zoom_` already do (100% maps one image px
to one logical px). Use a named `kCanvasRevealMarginPx = 96.0` (1 inch) with a
`ponytail:` note. Clamp per axis (slide the minimum) rather than recentering, so
the artwork does not jump under the cursor. Do not clamp in the read paths
(`widgetToImage`/`paintEvent`); `offset()` stays authoritative for the navigator
and tests. The present cache keys on image and zoom only, so a clamped pan does
not rebuild it.

**Specs/tests.** Add a `canvas-tools` requirement for the visible margin. The
existing pan assertions must change because panning past the margin now clamps:
`canvas_centre_offset` (exit 64), `canvas_middle_pan_delta` (exit 65) in
`selftest.cpp:1811-1864`, and the `zoom/pan transform wrong` check at
`selftest.cpp:6702-6722`.

## 19. Workspace scrollbars to pan the canvas

**Root cause.** The workspace has no scroll area. `ImageView` is the tab page
inside a `QTabWidget` inside the frame splitter (`frame.cpp:286,324,46-49`), and
panning is middle-drag, Hand-tool left-drag, and the navigator proxy only. No
scrollbar maps `offset_`/`zoom_` to a range. The theme already styles
`QScrollBar` (`theme.cpp:207-215`), so bars inherit the dark theme.

**Candidates.**
1. (Recommended) A thin container around `ImageView` with two `QScrollBar`
   children whose range is the same `offsetRangeFor` arithmetic as issue 18,
   value derived from `offset_`, `ScrollBarAsNeeded` so they hide when the
   document fits, and a new `viewChanged`/`panChanged` signal so they follow
   pan/zoom/fit/Navigator. `offset_` stays the single source of truth; the bars
   are a projection, never a second state store.
2. `QScrollBar` children wired straight to `panBy` without the shared range.
   Less code but duplicates the range math and can disagree with the clamp.
3. Wrap `ImageView` in `QAbstractScrollArea`. Rejected: it assumes a fixed
   content size in a viewport, fighting the transform canvas and the navigator's
   use of `size()` as the viewport.

**Shared primitive.** Issue 18 and issue 19 must share one
`offsetRangeFor(...)` helper; two independently written clamps will drift.

**Specs/tests.** New `canvas-scrollbars` capability (or a `document-tabs`/`
canvas-tools` requirement), plus `navigator-panel` sync and a
`workspace-persistence` decision on whether zoom/offset persist (currently they
do not; `docs/dev/canvas-view-spec.md:32-34` says they should). New checks:
pan in each direction and assert the margin intersects; scrollbar range/value
round-trip; bars hidden when the document fits.

## Cross-cutting notes

- **Spec conflicts to escalate before coding.** Issue 7 (zoom-scaled step,
  plain arrows moving a layer) contradicts `docs/dev/canvas-view-spec.md:141`
  and `docs/02-ui-ux/keyboard-shortcuts.md:389-392`. Issue 4's "always
  Background" rule needs a transparency decision. Issue 19 needs a decision on
  whether zoom/offset or scrollbar position is persisted. These belong in an
  OpenSpec proposal rather than a guess.
- **OpenSpec work.** Issues 3, 4, 7, 8 and 17 need MODIFIED requirements
  (`document-tabs`, `image-import`, `canvas-tools`/`edit-history`,
  `paint-engine`, `brush-tools`/`tool-framework`). Issues 10, 12 and 15 are
  code-vs-spec mismatches. Issues 6, 9 and 11 suggest a new `layer-locks`
  capability plus `layers-panel` amendments. Issues 13/14/16 share a new
  `numeric-fields` capability, and 18/19 a `canvas-scrollbars` capability (or a
  `canvas-tools` requirement).
- **Shared primitives, build once.** `NumericField` absorbs issues 13, 14 and 16
  (with `PercentField` as thin configuration); the existing `JumpSlider` becomes
  shared for issue 15; and one `offsetRangeFor(...)` helper serves both the pan
  clamp (18) and the scrollbar range (19). Building these separately is the main
  drift risk in this list.
- **Self-test codes are append-only** and the exit code is the failure identity
  (`AGENTS.md`). The highest code observed during this investigation is 296, so
  new checks start at 297 (re-verify, agents are editing concurrently). New
  checks must respect `scripts/file-size-allowlist.txt`. The pan clamp (18) will
  change existing assertions in `canvas_centre_offset` (64) and
  `canvas_middle_pan_delta` (65).
- **Docs guard.** Any `docs/` edit needs `TASK-ALLOWS-DOCS` in the commit
  message, per `AGENTS.md` rule 1. This note itself is such a change.

## Suggested sequencing

1. Quick, spec-backed wins: issue 10 (one line), issue 12 (one handler),
   issue 2 (gate the scratch document), issue 11 (delegate fill).
2. Numeric input family: build `NumericField` (issues 13, 14, 16), share
   `JumpSlider` for the navigator/color sliders (issue 15), then migrate the
   options bar and dialogs (issue 13/14 call sites).
3. Canvas view family: the `offsetRangeFor` clamp (issue 18) first, then the
   scrollbars derived from it (issue 19); update the pan assertions.
4. Persistence correctness: issues 1 and the restore half of 2.
5. Import identity: issues 3 and 4 (needs the transparency decision).
6. Locks and interactions: issues 6, 9, 5, and the brush-size circle (17).
7. Larger work: issue 7 (needs the shortcut decision) and issue 8 (engine
   region work; the highest user-visible payoff at 4000x4000).
