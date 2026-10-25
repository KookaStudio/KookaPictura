# Proposal

## Why

Four open selection-tool bugs share a pattern: state that belongs to one tool or one view is held globally, and the preview/outline paths recompute or clip full-document data where a cheap, local answer exists. Quick Selection leaks its combine mode into the marquee (#913) and recomputes a full-document flood on every move (#914); the committed outline disappears at the canvas border under fractional pan/zoom (#912); and selection tools have no canvas context menu (#884). Fixing them together keeps one verification pass over the selection stack and removes the shared-state seam behind two of the four.

Refine Edge's large-document crash (#915) was split out: no bounded repro was obtained, so its spec delta and verification move to the follow-up and this change does not claim the behavior.

## What Changes

- **#913 — option isolation.** Give Quick Selection its own option group, so its New-to-Add switch no longer changes the Marquee's combine mode. The options bar and persisted options follow the active tool's own group. Quick Selection still switches to Add after its first selection.
- **#914 — Quick Selection move cost.** Do not recompute a full-document flood for a dab that lands inside the already-selected region. Measure the press and move costs (composite, flood, combine, outline retrace) on a 4000×4000 document, set a latency target, and leave one-history and cancel semantics untouched.
- **#912 — committed outline at the border.** Keep the committed marching-ants outline visible when it lies exactly on the canvas border by letting the cosmetic stroke draw on both sides of the clip at fractional pan/zoom. The traced contour does not change.
- **#884 — selection canvas menu.** Add the selection-tool canvas context menu (Deselect, Inverse, Feather…, Refine Edge…) to the rectangular and elliptical marquee, lasso, polygonal and magnetic lasso, Magic Wand, and Quick Selection. It opens only with an active selection and reuses the registered Select commands. Transform Selection stays out until #526.
- **#915 — split out.** No bounded repro was obtained, so the Refine Edge large-document requirement, its delta, and its verification move to the follow-up. This change makes no claim about that crash.
- No new dependencies, no document-format change, no `docs/` edits. The `ToolServices` command-dispatch port is internal to the app.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/quick-selection`: the stroke requirement gains the intra-region no-op and states that the New-to-Add switch is the tool's own mode, not a mutation of a shared group.
- `tools/selection-outline-performance`: the committed outline must stay visible where it coincides with the canvas border across pan and zoom.
- `ui/canvas-context-menus`: selection tools gain a canvas menu, gated on an active selection, offering the four Select commands.

## Impact

- App C++: `crates/pictura-app/cpp/` — `tool_wand.cpp`, `tool_selection.cpp`, `tool_magneticlasso.cpp`, `tools.cpp`, `tools.h`, `tools_selection_move.cpp`, `tool_services.h`, `tool_options.h`, `options_bar.cpp`, `options_bar.h`, `tool_options_json.cpp`, `image_view.cpp`, `frame_build.cpp`, and the matching Qt tests.
- App Rust: `crates/pictura-app/src/cxxqt_object/impl_selection/quick.rs` and `impl_selection/bridge.rs`.
- Specs: `tools/quick-selection`, `tools/selection-outline-performance`, and `ui/canvas-context-menus`.
- Verification: `tst_tool_commit`, `tst_selection_preview`, `tst_selection_api`, `tst_canvas_context_menus`, `tst_tool_options_persistence`, plus one ignored 4000×4000 Quick Selection profiler.
- Dependencies: none added; existing Qt/Rust libraries only.
