# Design: Paint, selection and type tool bugs

## Approach

### #863 Auto Erase leak

`BrushToolHandler` carries `aliased_` to select Pencil rasterisation and ORs
`paint_->aliased()` for a pencil preset
(`crates/pictura-app/cpp/tool_brush.cpp:24-29,48-52`). Gate only the auto-erase
argument: `aliased_ && paint_->autoErase()`. The rasterisation argument stays
`aliased_ || paint_->aliased()`. Keep the flag in the shared `PaintOptions`:
persistence and the Pencil-only checkbox stay as they are, and no state
migration is needed.

### #864 healing mask target

The mask document the paint tools edit is the single-channel grayscale
`mask_document_coverage`
(`crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs:37`), while
`heal_layer` requires R, G, and B planes
(`crates/pictura-paint/src/healing/layer.rs:20`). Do not build a second healing
engine. `begin_heal` checks `active_mask_target`
(`crates/pictura-app/src/cxxqt_object/healing.rs:114`) and refuses before
creating a `HealStroke`; `tool_healing.cpp` adds a mask-target branch to
`refuseHeal` (checked before the lock branch) using `mask_edit_target_get`
(`crates/pictura-app/src/cxxqt_object/impl_layers/layer_masks.rs:317`), so the
message names the target. That requires including
`pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h`. Refusal records
no history and touches neither the layer nor the mask.

### #865 phantom history and stale RGB

`Stencil::composite` (`crates/pictura-paint/src/stroke.rs:912`) returns whether
the bytes it writes differ from the current plane bytes. `DabRaster::band`
(`crates/pictura-paint/src/stroke/raster.rs:44`) grows its bounding box only for
pixels where that returns true, so a dab that leaves every byte unchanged cannot
set `painted` or grow `dirty`. In `apply_per_dab`
(`crates/pictura-paint/src/stroke.rs:732`), compare each pixel before writing
it to the layer and grow the dirty rectangle only around changed bytes.

For RGB zeroing, `blend_pixel`'s `Clear` branch
(`crates/pictura-paint/src/stroke/blend.rs`) returns RGB 0 when the composited
coverage is 1.0 and keeps the existing RGB below that; `write_band`
(`crates/pictura-paint/src/stroke/raster.rs:160`) then writes the RGB planes for
Clear instead of skipping them when the layer has an alpha plane. A layer
without alpha still writes nothing, so Clear stays a no-op there. The GPU dab
shader's Clear branch (`crates/pictura-render/src/gpu/shader.rs`) zeroes the
word at the same coverage so a GPU stroke matches the CPU oracle, and
`patch_working_layer` skips unchanged rows so a GPU dab that changes no byte
stays out of history too. The existing `behind_leaves_opaque_rgb_unchanged` test
(`crates/pictura-paint/src/stroke/tests.rs:418`) changes from expecting a dirty
rectangle to expecting `None` from `finish()`, which is what keeps history empty.

### #866 Mixer Mode and Opacity

`begin_mixer_brush` (`crates/pictura-app/src/cxxqt_object/paint_tools.rs:383`)
takes `opacity` and `mode` like the other paint beginnings;
`tool_mixerbrush.cpp:54` passes `paint_->opacity()` and `paint_->mode()`, and
`MixerBrush::dab` (`crates/pictura-paint/src/mixer.rs:91`) folds `cfg.opacity`
into the deposited weight and routes the result through the shared
`blend_pixel(cfg.mode, ...)` (`crates/pictura-paint/src/stroke/blend.rs`). Reuse
the five existing `PaintMode` values rather than introducing a second blend
enum; Clear on a Mixer is odd but consistent, and the issue only requires
Multiply.

The Mixer options page (`crates/pictura-app/cpp/options_bar_paint.cpp:243`) adds
an `optionsMixerMode` combo and an `optionsMixerOpacity` field bound to the
shared `PaintOptions`, because Mode and Opacity are stroke-wide paint options;
the handler passes both to `begin_mixer_brush`. `apply_per_dab` continues to
write the already-composited RGBA with Normal.

### #867 Contract and Border at the edge

`Selection::morph` (`crates/pictura-select/src/lib.rs:360`) stops clamping taps.
A tap outside the canvas contributes 0 in both passes, which erodes an
edge-touching selection inward and leaves dilation unchanged. `border`
(`crates/pictura-select/src/lib.rs:326`) inherits the fix because it is
`expand(ceil(w/2))` symmetric-difference `contract(floor(w/2))`.

This reverses the published "Canvas edge is exempt" scenario in
`tools/selection-model`; the delta replaces it with "Contract pulls away from
the canvas edge" and adds the previously unspecified Border band requirement.

### #868 Quick Selection stroke

The accumulated selection must survive the whole drag, so the selection work
moves into Rust. Add `quick_select_preview` (computes the wand region and
applies it with `Edit::Meta` through `preview`
(`crates/pictura-app/src/cxxqt_object/edit.rs:169`), recording nothing) and
`quick_select_commit` (records one `Edit::Meta` "Selection" state). The C++
handler (`crates/pictura-app/cpp/tool_wand.cpp:40-86`) calls preview on press
and move, commits on release only when a dab applied, and captures the gesture
mode at press with `resolveSelectionMode`. For a New-mode gesture, the first dab
uses New and the handler switches its drag mode to Add for later dabs so they
extend the stroke's accumulated selection; an Add or Subtract gesture keeps its
mode for the whole stroke. After a New-mode stroke's first selection, set the
selection options group's combine mode to Add so the next stroke adds. Snapshot
`session.selection` and `session.deselected_selection` at press and restore both
from `onSessionCancelled` and `onDeactivate`, so a cancelled preview leaves no
partial selection. Keep
`apply_selection` (`crates/pictura-app/src/cxxqt_object/impl_selection.rs:547`)
as the single combine implementation: split the mutation from the history
record rather than adding a second combiner. Alt keeps the existing
`resolveSelectionMode` subtract behavior, which applies only when a selection
exists at press.

### #869 Refine Edge units and preview scale

`boundary_distance` (`crates/pictura-select/src/refine.rs:167`) divides the 3-4
chamfer by 3, the orthogonal weight, so axial distances are exact pixels. In the
preview path, after `shrink` produces the proxy image, scale `radius`, `smooth`,
and `feather` by the proxy's linear factor
`f64::max(image.width as f64 / doc.width as f64,
image.height as f64 / doc.height as f64)` (the proxy's linear shrink factor, up
to the per-axis rounding in `shrink`), rounding to at least 1 when the source
value is positive. Contrast and
Shift Edge are percentages and stay unchanged. Extract the scaling into a small
pure helper so it is unit-testable without a window. Add a Qt case that drives
the preview and the commit of the same edge at one Radius and compares their
edge widths at proxy scale, so the two paths agree within proxy tolerance.

### #870 path Baseline Shift

`layout_path` (`crates/pictura-render/src/type_path.rs:99`) reads
`RunStyle::of(bundled, &spec.character).rise`
(`crates/pictura-render/src/type_style.rs:193`) and offsets each glyph by
`-rise * normal`, where `normal = (-tangent.y, tangent.x)`.
`caret_stops` (`crates/pictura-render/src/type_path.rs:327`) applies the same
offset. `placement` and `render_path` consume the layout glyph positions, so
the layer/text bounds and ink follow; the caret follows its own offset.

## Order

`#863 → #864 → #865 → #866 → #867 → #868 → #869 → #870`.

## Risks

- `#865` changes the shared raster path used by every paint tool; land its
  engine tests before the Qt cases.
- `#867` flips a published selection rule. The spec delta must be reviewed with
  the code, not treated as an implementation detail.
- `#868` adds a second selection mutation path (preview plus commit); keep
  `apply_selection` as the single combine implementation and only split
  recording from mutation.
- `#868` changes the shared `SelectionOptions` combine mode
  (`crates/pictura-app/cpp/options_bar.cpp:85`), so the marquee and wand also
  read Add afterwards. The mode buttons are checked only when a page is built
  (`crates/pictura-app/cpp/options_bar.cpp:393`), so the Quick Selection button
  does not visibly flip to Add without a re-sync.
  ponytail: the mode stays shared; give Quick Selection its own options group if
  per-tool mode matters.

## Test strategy

Reuse the existing suites; no new `tst_*.cpp` file and no CMake test entry.

| Area | Test |
|---|---|
| #863 | `tst_options_bars_parity::pencilAutoEraseDoesNotLeakToBrush` |
| #864 | `tst_mask_edit::healingRefusesMaskTarget` |
| #865 | `pictura-paint::noop_stroke_adds_no_history`, `pictura-paint::clear_zeroes_rgb`, `pictura-paint::clear_keeps_rgb_below_full` |
| #866 | `pictura-paint::mixer_applies_mode_opacity`, `tst_options_bars_parity::mixerExposesModeOpacity` |
| #867 | `pictura-select::contract_all_shrinks`, `pictura-select::border_edge_has_inner_band` |
| #868 | `tst_tool_commit::quickSelectionOneStatePerStroke`, `tst_tool_commit::quickSelectionCancelRestores` |
| #869 | `pictura-select::refine_distance_is_pixels`, `pictura-app::preview_params_scale_with_the_proxy`, `tst_refine_edge::previewMatchesCommitWithinProxyTolerance` |
| #870 | `pictura-render::type_on_path_applies_baseline_shift` |

## Constraints

- Follow `docs/dev/testing-conventions.md`: Qt Test for GUI, std `#[test]` for engine.
- Do not weaken or delete an existing test; update a baseline only with a before/after assertion.
- Keep each file under its size cap.
