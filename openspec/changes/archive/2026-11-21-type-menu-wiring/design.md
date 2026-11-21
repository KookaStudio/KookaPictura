# Design

## Context

See `proposal.md` for motivation and the spec deltas for the contract. The relevant current state:

- `crates/pictura-app/cpp/menu_type.cpp` already wires point/paragraph conversion, Warp Text, and Replace All Missing Fonts over `api/type_api`.
- `crates/pictura-app/cpp/panels/layers_panel_type_menu.cpp` already has the AA/orientation update pattern and two disabled outline placeholders.
- `crates/pictura-render/src/type_warp.rs` already walks glyph outlines through `flat_layout`, `RunStyle`, swash `scale_outline`, the warp map, and decorations.
- `add_shape_layer`, `set_layer_shape_paths`, `delete_paths`, `render_text_layer`, `type_layers_editable`, and `re_render_type_layers` exist.
- The Type preferences (`typeShowAsianOptions`, `typeMiddleEastern`) are stored in the application profile. `font_combo.cpp` has one hard-coded preview size. This change adds the Font Preview Size preference.
- The catalogue is `crates/pictura-script/src/command_catalogue/entries.rs`; the C++ header and golden tables drift-check against it.

## Goals / Non-Goals

**Goals:**

- One shared text-to-outline path for both structural commands.
- In-place Convert to Shape so layer appearance survives.
- Type menu rows that reuse the existing type APIs and report real check/enable state.
- A testable, offline find/replace with one undoable state.
- A catalogue that matches the visible menu and the docs.

**Non-Goals:**

- Adobe Fonts, Typekit, Adobe APIs, or Match Font (out of scope; Google Fonts is #1442).
- New dictionaries or Check Spelling UI (#1443).
- Windows LCD/Windows anti-aliasing, the Middle Eastern & South Asian composer, and a Resolve Missing Fonts dialog.
- Variable-font UI or custom mesh warps.

## Decisions

### 1. Outline extraction lives in one render module

New `crates/pictura-render/src/type_outline.rs` exposes `outline_paths(spec) -> Option<Vec<Subpath>>` and an outline refusal (`NoFace`, `NoOutlines`, `Empty`).

- Unwarped, on-path, and area type: build curves directly from the swash verbs (`MoveTo`/`LineTo`/`QuadTo`/`CurveTo`/`Close`) into `PathPoint` anchors and handles. Quadratics are elevated to cubics exactly; this keeps anchors low and editable. (`type_warp` flattens to polylines for its raster path; this walk does not.) Unwarped type uses `flat_layout`; on-path type uses `type_path::layout_path` and the glyph rotation; area type uses `flat_layout` and skips the render-time clip.
- Warped type: reuse the flattened contours from `type_warp::geometry` (made `pub(crate)`) as closed corner-anchor subpaths. A non-affine warp cannot preserve curves without fitting, and the warp raster path already accepts the polyline approximation.
- Decorations (underline/strikethrough) are separate contours, as in the warp geometry.
- Faux-bold emboldening and the styled run (`RunStyle`, fallback faces) come from the existing walk.

Alternative considered: always flatten. It is less code but produces hundreds of anchors per glyph and loses editable curves for the common unwarped case.

### 2. Create Work Path authors the single work path

A `type_tools` bridge function reads the layer spec, computes `outline_paths`, builds a `VectorPath` with `add_subpath`, and sets `doc.work_path` inside one `view.edit("Create Work Path", |_| Edit::Meta)` call. `Edit::Meta` captures and notifies without compositing; the C++ handler then calls `paths_set_active(*view, -1)`, `showWorkPathOverlay`, and refreshes the Paths panel.

Alternative considered: a separate `paths` API. The outline walk is type-specific; the app bridge is the smallest seam.

### 3. Convert to Shape mutates the type layer in place

A render document op `convert_type_to_shape(doc, path)`:

1. Reads the type spec and computes the outline before mutating.
2. Clears the type content (`TySh`, type tool, applied styles, type overrides, type locks), pixel channels, `raw_channels`, and `source_channels`, sets `pixel_data_irrelevant`, and writes a solid-colour adjustment for the type fill colour.
3. Sets the canvas rect (as `add_solid_fill` does) and calls `set_layer_shape_paths` with all outline subpaths.
4. Leaves the name, blend, opacity, raster layer mask, clipping, and effect blocks untouched; the glyph outline becomes the vector mask, replacing any existing vector mask.

The bridge records one `Convert to Shape` state with `structural(before, after)` damage. `add_shape_layer` + delete was considered, but it loses layer effects, masks, and stack position and makes the post-delete path index fragile.

The model carries one character colour per layer, so "first character's colour" is the layer's `character.fill_color`; `add_solid_fill` drops alpha, and the shape keeps the layer's Uniform Fill as opaque (documented ceiling).

### 4. Type menu rows reuse `readLayer` / `updateLayer`

AA, orientation, and OpenType are one-shot read/modify/`updateLayer` operations, recording the existing `Edit Type Layer` state. The Layers row menu's local `update` lambda is the pattern; the Type menu does not invent a second type API.

- Catalogue `checkable` becomes true for AA (5), orientation (2), Font Preview Size (6), Language Options (3), and OpenType (10); the C++ side registers checked providers.
- Layer-mutating rows require an editable type layer (`type_api::layersEditable`): AA, Orientation, OpenType, Rasterize Type Layer, Update All, Load Defaults, Paste Lorem (selected layer), and the outline commands. Panel toggles and preference rows (Glyphs, Language Options, Font Preview Size, Save Defaults) are not gated on it. The layer-mutating rows other than Paste Lorem Ipsum are also disabled while a Type tool session is open.
- Language Options mirror the two existing preference flags; `Default Features` clears both. The menu does not change the preferences' independent-checkbox semantics.
- Font Preview Size is a typed integer preference (None..Huge). `font_combo` reads a process-wide setter; the delegate's `sampleFont` uses the selected point size and `sizeHint` includes it so the sample is not clipped.
- The Type section of the TOML preferences gains `fontPreviewSize` and `typeDefaults`. Both load before the first document renders.

### 5. Default type styles use the typed preferences store

`Save Default Type Styles` reads the active layer with `type_api::readLayer` and stores `font`, `character`, and `paragraph` as a versioned base64 payload in the Type preferences. It records no document state. `Load Default Type Styles` builds a spec from the stored attributes plus the layer's own text and placement, then calls a dedicated bridge that records `Load Default Type Styles` (not the generic `Edit Type Layer`); the row is disabled in a non-editable document or during a Type session. The shell serializes the style attributes and saves the resulting string through the typed preferences store.

### 6. Paste Lorem Ipsum follows CC

Enabled when a Type tool session is active or the current layer is type. A session insert uses `TextSession::insertText` (the existing text-field model, no document state until commit). A selected-layer insert appends the block at the end of the layer's text as one `Paste Lorem Ipsum` state. No layer and no session means disabled; we do not create a new text box.

### 7. Find And Replace is a pure matcher plus one history state

`pictura-core` gets a pure `replace_matches(text, find, replacement, options) -> (String, usize)`; ASCII case folding and an alphanumeric/underscore whole-word boundary are deliberate ceilings. Replacements are not rescanned. `Forward` on scans the original text left-to-right and picks the first non-overlapping match; off scans right-to-left and picks the last, so `aa` → `X` in `aaa` gives `Xa` or `aX`. Replace All is disabled for an empty Find; a replacement that empties a layer uses the blank-allowed re-set path and leaves an empty type layer. The command is enabled only for an editable document and, with Search All Layers off, a type current layer; it is disabled during a Type session.

`ui/find-and-replace-text` is a new window-modal Qt dialog; `edit.find.and.replace.text` is added to `modal_ids()`. The bridge collects matching type layers (skipping fully locked ones), applies the matcher to each text, and calls `replace_type_layer`; one `view.edit("Find And Replace Text", |_| Edit::Full)` records the result. `Edit::Full` is acceptable for a one-shot document-wide text edit; a regional union is not worth the complexity.

### 8. Catalogue and spec cleanup

- Remove `type.check.spelling`, `type.find.and.replace.text`, and `type.extrude.to.3d` from `entries.rs`; keep the Edit spellcheck row (disabled until #1443) and the Edit find/replace row.
- Add `type.panels.glyphs`, `type.language.options.default.features`, and the ten `type.openType.*` rows.
- Set `checkable` on the radio/toggle rows; set `implemented` on every row this change wires, including the already-wired convert/warp/replace rows, so the catalogue and `list_commands` are honest. Update `catalogue.tsv`, and regenerate `const_names.tsv` and `command_ids.h` from `const_names.rs`, and the stable-shape counts.
- Correct the command-registry requirement that says the Type rasterize entry has no handler, and add the `ui/layers-panel` MODIFIED delta that enables the two outline rows for type rows while keeping them absent for non-type rows.
- Update `ROADMAP.md`, the parity register UI-6, and the type-tools doc's ceilings; docs commits carry `TASK-ALLOWS-DOCS`.

### 9. Test seams

- Rust: `type_outline` unit tests (outline vs rendered alpha-ink bounds, not the `type_placement` layout box; contour counts; warped/on-path/area; a transformed point/paragraph case), document-op conversion tests (kind, name, effects, undo), and pure matcher tests (case, whole word, forward, overlap).
- Qt: `tst_type_menu` (row enablement, check states, one-state behaviour, work path/shape end to end, lorem, preview size, language options, defaults), `tst_find_replace` (dialog, scope, options, no-op), and updates to `tst_layers_row_menu` and catalogue assertions.

## Risks / Trade-offs

- **Area outlines are un-clipped** → documented ceiling; the renderer's area type is already an approximation.
- **Warped outlines are polylines** → the warp raster path uses the same approximation; note it in `docs/03-tools/type-tools.md`.
- **Find/replace case folding is ASCII** → documented ceiling; whole-word boundaries are alphanumeric/underscore.
- **In-place conversion must clear every type block** → tests cover effects, raster masks, name, undo, and PSD round-trip; a missed block would keep the layer type-like.
- **The solid-fill writer stores an opaque colour** → a non-opaque type fill becomes opaque; document the ceiling and test the opaque case.
- **Imported multi-run type** → outlines use the first style run; document the ceiling.
- **Two independent language preferences can both be on** → the menu mirrors them; `Default Features` clears both, and the menu is not a strict radio group.
- **Font preview sizes are approximate** → the six labels map to point sizes; tests assert None removes the sample and Huge is larger than Small.
- **Catalogue flag flips ripple into goldens and counts** → deliberate; update the captured tables in the same change.
- **No new dependencies** → the find/replace matcher and the outline walk use existing crates.

## Migration Plan

The legacy preferences migration recognizes `fontPreviewSize` and `typeDefaults` in session JSON. Profiles without the keys load Medium preview and an empty default type style. The catalogue removals are menu-only and do not affect existing documents. Rollback is the usual revert of the change; PSD files are untouched.

## Open Questions

None material to this change. The deferred surfaces are tracked: Google Fonts (#1442), Check Spelling (#1443), CC-only language/AA rows, and Match Font.
