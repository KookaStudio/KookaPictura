# Design

## Context

See `proposal.md`, "Why". Each unit has the same shape: a table or formula lives
in C++, and the engine either has the same data (blend modes, adjustment kinds)
or should own it (colour conversions, tool rows). The bridge precedent is
`cxxqt_object/command_catalogue.rs`. That is a free-function `#[cxx_qt::bridge]`
returning `Vec<CommandSpecRow>`, which `commands.cpp` reads once. None of these
tables depend on `PictureView` state.

These constraints shape the design:

- cxx-qt can return `Vec<shared struct>`; `command_catalogue` already does.
- `pictura-app` does not depend on `pictura-color`. It reaches lcms2 only
  through `pictura-render` and `pictura-filters`.
- `color_picker_dialog.cpp` is 1181 LOC against a 1200 cap, and `toolbox.cpp` is
  1018. Both changes must shrink or hold these files.
- The view-seam spec makes `type_api` the exclusive surface of the type unit and
  rejects 1:1 forwarders. Folding `type_api` back would break the first rule, and
  keeping it as it is breaks the second.

## Goals / Non-Goals

**Goals:**

- One Rust source for each table or formula, read by C++ through one bridge
  call per table.
- The units land in the suggested order (#271, #269, #272, #274, #403). Each is
  one commit carrying its own issue number, and each passes its own tests.

**Non-Goals:**

- Paint-mode combos (options bar, Fill, Stroke, Stamp) and the Design System demo
  lists. They are a different mode set (Behind, Clear) and not layer modes.
- The out-of-gamut triangle and Info panel exclamation (#600, #604, #516),
  readouts that follow the document profile, and a CMYK working-space choice in
  Color Settings.
- The dialogs' control layout. Only their numbers move.
- #275, #277, #278, #280, #783, #359.

## Decisions

### D1. Static tables cross as cached row vectors

Each table gets a free-function bridge module (`blend_modes.rs`,
`adjustment_catalogue.rs`, `tool_catalogue.rs`) shaped like `command_catalogue`.
No new `pictura-core` accessor is needed for blend modes, because `LAYER_MODES`,
`to_psd_key`, `name` and `pictura_render::neutral_color` already exist. The
bridge row adds `has_neutral`, which replaces `layer_new_dialog.cpp`'s
`modesWithoutNeutral()` key set.
On the C++ side, one accessor per table converts the rows once into a
function-local `static const` vector and hands out references. Examples are
`pictura::blendModes()` in `blend_modes.h` and the existing `toolInfo(ToolId)`.
Generated headers are included only from that accessor's `.cpp`.

*Alternative considered:* call the bridge at every use. It would cross the
bridge repeatedly for immutable data and spread generated-header includes, so
it was rejected.

### D2. Colour conversions live in `pictura_color::model`

- The scalar core of the codec's planar `rgb_to_lab`/`lab_to_rgb` moves out of
  `pictura-codec/src/color_mode.rs`, along with its helpers (`lab_f`,
  `lab_fwd`, the sRGB encode/decode and `XYZ_D50_TO_LINEAR_SRGB`). The new
  `pictura_color::model::srgb_to_lab` and `lab_to_srgb` work per pixel in f64,
  D50 and Bradford; that code already matches lcms2 within 1 LSB.
  - The codec's planar functions and their re-exports keep their signatures and
    call the core. Their callers are render `mode/planes.rs`, codec
    read/write/pdf and app `gradient_map.rs`.
  - The formula then exists only once. `pictura-adjust` keeps its private Match Color copy; it has its own
  oracle and folding it is out of scope.
- `CmykConverter` builds the sRGB↔working-CMYK lcms2 transforms once and keeps
  them in a `OnceLock`. It uses f64 pixels and
  `Flags::NO_CACHE | Flags::BLACKPOINT_COMPENSATION`, which gives
  `DisallowCache`, so the transform is `Sync`. f64 keeps white at exactly 0
  ink; an 8-bit transform rounds white up to 1%. The
  intent is relative colorimetric with black-point compensation, which matches
  CS6's default Color Settings (North America General Purpose 2). It offers
  scalar `srgb_to_cmyk` and `cmyk_to_srgb`. Only `srgb_to_cmyk` is bridged; the
  picker's CMYK fields are read-only, and `cmyk_to_srgb` backs the round-trip
  test.
- The working profile is `Profile::coated_cmyk()`. **ponytail:** this is a
  modelled press, not SWOP, so CMYK numbers will not equal CS6's. Parity is
  structural (profile-mediated) and is not claimed numerically.
- A `color_models.rs` bridge exposes `srgb_to_lab`, `lab_to_srgb` and
  `srgb_to_cmyk`. HSB stays on `QColor`, which is already the standard
  transform.
- The picker's CMYK tooltip, "direct conversion without a press profile", is
  reworded to name the working profile.
- **Channels panel.** Its thumbnails show the document's plate split, the naive
  GCR the CMYK mode conversion writes (`rgb_to_cmyk_gcr`) and that
  channel-to-selection already mirrors in `impl_selection.rs`
  (`channel_value`). A profile CMYK there would disagree with the stored plates,
  so the thumbnails keep that split. The C++ `toCmyk`/`channelValue` are deleted
  in favour of a bridged `channel_plane(rgba, mode, row)` built on the Rust
  `channel_value`, which leaves one derivation and unchanged output.

*Alternatives considered:*

- Keep the naive CMYK. That contradicts `docs/07-color-painting/color-models.md`
  ("not by a fixed formula").
- Route Lab through lcms2 as well. It gives equal results within 1 LSB, but the
  analytic form is already tested and needs no profile handle.

### D3. Adjustment catalogue beside `ADJUSTMENT_DIALOG_KINDS`

`pictura-render/src/adjustment_catalogue.rs` holds `AdjustmentEntry`:

- `kind`
- `title`
- `image: AdjustmentsDialog | AdjustmentsDirect | ImageRoot | None`
- `layer: Ellipsis | Plain | None`
- `panel_icon: Option<&str>`
- `panel_menu: Option<&str>`, the hyphenated flyout text such as
  "Brightness-Contrast"
- `fill: bool`

These cover 25 rows in CS6 menu order: three fills, sixteen dialog kinds, the
three direct rows Desaturate, Equalize and Invert, and the three Auto rows.
Invert is one row that is both a direct command and a Plain layer entry.

- The menu structure (leaves, order, shortcuts, separators) already comes from
  the `pictura-script` command catalogue and `commands.cpp`. The adjustment
  catalogue does not duplicate it.
- C++ resolves each row's command id with `commandIdForPath` from the row's
  location and title, then attaches the handler.
- A `pictura-app` Rust test asserts that every row resolves to a
  `CommandSpec`, so a title drift fails a test instead of silently skipping a
  handler.
- `ADJUSTMENT_DIALOG_KINDS` has no production callers and the wrong Color
  Lookup position, so it is deleted.
- `helpers.rs`'s `adjustment_layer` names and test list now derive from the
  catalogue. `AdjustmentEditor.title` stays in the layout, which decodes by
  descriptor rather than by kind, and a test pins it to the catalogue title.
- HDR Toning, Match Color and Replace Color stay hand-wired, as the spec
  exempts them.
- Fill rows supply labels and order. Their actions stay `add_solid_fill`,
  `add_gradient_fill` and a disabled Pattern.

For ranges, the dialogs already receive `image_adjustment_page` rows that carry
min, max, default and decimals. A small helper
in `adjustment_dialog.h` (`AdjustmentDialog::paramSpec(page, key)`, with a
protected `spec(key)`) replaces each literal range. Where a hand-laid dialog
edits a value the Rust layout does not yet describe, the spec is added to the
Rust layout as a grouped or hidden entry. Three kinds of control keep their
shell logic, as the spec exempts them: the coupled Levels clamps, the Curves
point fields and the Black & White tint hue and saturation.

*Alternative considered:* a new per-key bridge call. It is redundant, because
the page rows are already fetched.

### D4. Tool catalogue in `pictura-script`

`pictura-script/src/tool_catalogue.rs` sits next to the command catalogue.
`pictura-app` already depends on that crate, and actions name tools by string
(`ToolStroke.tool`). Each row is `ToolSpec`:

- `name`
- `label`
- `group`
- `shortcut: Option<char>`, which crosses as a `QString` that is empty for none
- `cursor: ToolCursor`
- `hotspot`
- `hint`
- `modifier_hints: &[(key, text, command_id)]`
- `implemented`

`ToolCursor` is a small enum (arrow, cross, size-all, pointing-hand, …) mapped
to `Qt::CursorShape` in one C++ switch.

On the C++ side:

- `ToolId` and `kToolCount` stay as they are, because handlers switch on them
  and the registry sizes arrays by them.
- `tool_catalog.cpp` builds the `ToolInfo` table once from the rows. Its
  `const char*` fields stay, backed by `QByteArray` storage cached beside the
  table, so the call sites and the `%s` uses in `selftest.cpp` are untouched.
  `selftest.cpp` is at its ceiling.
- `allToolIds` and `toolHintEntries` derive from the rows.
  `implementedToolIds()` has no consumers and is deleted.
- The order test cannot compare against the table built from the rows, which
  would be true by construction. `tst_keyboard_shortcuts` instead keeps its own
  71-entry `{ToolId::Move, "move"}, …` table and compares it with the bridged
  rows.
- Shortcut overrides (QSettings) stay in C++.

*Alternative considered:* generate the `ToolId` enum from Rust. It needs a
build step for 71 identifiers, and the order test catches drift just as well.

### D5. Deepen `type_api` around values and sessions

- `TypeSpec { text, font, setting, character, paragraph }` replaces the
  five-argument tuple in every call that takes it: `previewRect`, `caretStops`,
  `previewRgba`, `baselines`, `overflows`, `pathOverflowPoint`, `localBounds`,
  `commitLayer`, `commitMask`, `updateLayer` and `commitEdit`. `LayerType`
  holds a `TypeSpec`.
- `readLayer` is the only layer read. `isType` and the `layerText`, `layerFont`
  and `layer*Setting` getters leave the interface.
- `EditSession` is move-only and holds a `QPointer<PictureView>`. Its owner
  keeps it in a `std::optional` or `std::unique_ptr` member: the Type tool for
  re-edits, and the Warp Text dialog for its preview.
  - `begin(view, path)` returns `std::optional<EditSession>`.
  - `abandon()` ends it without touching the layer, for an in-place document
    replacement that already restored it (`onSessionCancelled`).
  - `commit(spec)` or `cancel()` ends it. A failed commit also ends it, since
    Rust already shows the layer again.
  - The destructor cancels an open session, and does nothing when the view is
    gone.
- `WarpSession` works the same way. `preview` stashes the layer once, `commit`
  records "Warp Text", and `cancel` or the destructor restores the layer. The
  non-modal dialog's three connections in `frame_menus_type.cpp` share one
  `std::shared_ptr<WarpSession>`, so the session dies with the dialog.
- `layers_api.cpp` stops including `type_tools.cxxqt.h` and `warp.cxxqt.h`, and
  calls a `type_api::isWarped` instead.
- `characterStyles(view)` and `paragraphStyles(view)` return
  `QVector<…Style>` from one new Rust call per library
  (`type_character_styles` / `type_paragraph_styles` in `type_tools.rs`). The
  nine count, name, font and setting getters are deleted from the bridge.
  `findStyle` falls back to the default attribute sets for an unknown name.
- `type_frame.h` and `panels/character_panel.h` include `api/type_api.h` instead
  of `type_tools.cxxqt.h`. The named #276 exception (`type_api.h` re-exports
  the shared setting structs) stays, and is the only one.
- The decision is recorded in `docs/dev/refactor-view-seam.md`: deepen, why
  fold-back was rejected, and the exception.

*Alternative considered:* fold `type_api` back. That would re-expose
`type_tools.cxxqt.h` to about 35 files and contradict the exclusive-surface
requirement, so it was rejected.

## Risks / Trade-offs

- [CMYK and Lab readout values change] → The change is intended and matches the
  contract. Update `tst_info_panel` and `tst_channels_panel` expectations only
  where they encode the naive or D65 numbers, and list each changed expectation
  in the PR body.
- [lcms2 cost on cursor moves] → One pixel per move through a cached transform
  costs microseconds. Thumbnails use the buffer call. Do not build a transform
  per call.
- [A hand-laid dialog edits a value the Rust layout lacks] → Add the spec to the
  Rust layout, with a Properties-panel visibility flag if needed, and cover it
  with an `adjustment_params_tests` case.
- [Static C++ caches initialised before Rust is ready] → Rust functions have no
  runtime setup, and the caches are function-local statics, so the order is
  safe.
- [Deleting the `type_api` getters breaks about 20 test suites] → Migrate each
  test call to `readLayer`, `TypeSpec` or the style rows mechanically, keep every
  assertion, and say so in the PR.
- [Tight test files] → `tst_layers_panel.cpp` (1399/1400) and
  `tst_command_tree.cpp` (1396) take no new slots. New layer-side cases go in a
  new `tst_blend_mode_list` suite and in `tst_adjustments_panel`.
  - `src/cxxqt_object.rs` is at 1190/1200; the four `mod` lines fit.
  - `type_tools.rs` was at 1101; replacing its nine style getters with the two
    snapshot calls brings it to about 1090.
- [`ToolInfo` field types] → `const char*` is kept, backed by the cached rows'
  storage.
- [File-size caps] → Each unit deletes more C++ than it adds. Run
  `check-file-size.sh` per unit.

## Migration Plan

The units go in this order: #271, #269, #272, #274, #403. Each commit deletes its
C++ table in the same commit that adds the bridge, so no dual source survives a
commit. Rollback means reverting the unit's commit. The change archives in the
same PR, under the zero-open rule.
