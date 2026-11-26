# Proposal

## Why

The selection tools rasterise every primitive at the pixel centre, so their edges are binary and the Anti-alias controls are visible but disabled. The same gap leaves Photoshop's 56-channel document limit enforced only by the PSD codec, so the app can add a 57th channel and fail later at save. This closes SEL-6 in the docs↔code parity register (#614).

## What Changes

- Add anti-aliased selection primitives in `pictura-select`: the elliptical marquee uses analytic edge supersampling; the lasso, polygonal lasso, and magnetic lasso reuse the existing anti-aliased scanline coverage in `pictura_core::shape`.
- Add Magic Wand anti-aliasing as an inferred boundary colour-distance ramp on the binary flood result. Anti-alias off, or Tolerance 0, stays binary.
- Enable and persist the Anti-alias option for the Elliptical Marquee and the three lasso tools, and enable it for the Magic Wand; `Sample All Layers` remains disabled with its reason.
- Enforce a 56-channel document budget in the core document model, share the constant with the PSD codec, and refuse New Channel / Save Selection at the limit without recording history.
- Disable New Channel at the cap and show a non-modal status-bar refusal when Save Selection is invoked at the cap.
- Update the parity register, roadmap, and resume notes once shipped.

Not in scope: spot-channel kinds, the Paths panel's missing Anti-aliased checkbox (its coverage is already anti-aliased), and per-tool Anti-alias sync among the three lassos (they already share the selection options group, like Feather).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/shape-selection-tools`: the Elliptical Marquee and the lasso tools offer a functional Anti-alias option; the disabled-ceiling requirement and its scenario are replaced.
- `tools/magnetic-lasso`: the Magnetic Lasso's shared lasso commit honours the Anti-alias option.
- `tools/selection-model`: add the anti-aliased primitive coverage contract (fractional boundary coverage, binary when off, 50 % contour tolerance).
- `tools/selection-tools`: the Magic Wand's Anti-alias is modelled as a boundary colour-distance ramp; only `Sample All Layers` remains disabled.
- `tools/selection-channels`: add the source-mode-aware 56-channel document budget and its refusal behavior in New Channel and Save Selection.
- `ui/channels-panel`: New Channel is disabled at 56 channels and Save Selection refuses with a clear message.
- `ui/tool-options-persistence`: the selection and elliptical Anti-alias values round-trip, and the Magic Wand owns its own options group.

## Impact

- `crates/pictura-core/src/channels.rs` (new budget module), `crates/pictura-core/src/lib.rs` (module declaration).
- `crates/pictura-codec/src/common.rs` (shared channel constant).
- `crates/pictura-select/src/lib.rs` (anti-aliased ellipse and polygon builders), `src/wand.rs` (wand ramp).
- `crates/pictura-app/src/cxxqt_object/impl_selection/bridge.rs` (free anti-alias commit functions) and `impl_selection/bridge/imp.rs` (implementations), `channels/imp.rs`, `impl_selection.rs`.
- `crates/pictura-app/cpp/tool_options.h`, `tool_options_json.cpp`, `options_bar.cpp`, `tools.cpp`, `tool_selection.cpp`, `tool_magneticlasso.cpp`, `tool_wand.cpp`, `panels/channels_panel.cpp`, `frame_menus_select.cpp`.
- New tests: `crates/pictura-select/tests/aa.rs`, `crates/pictura-codec/tests/psd/channel_budget.rs` (cap tests), `tst_selection_aa.cpp`, plus `tst_channels_panel` and `tst_tool_options_persistence` cases.
- Docs: `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`.
