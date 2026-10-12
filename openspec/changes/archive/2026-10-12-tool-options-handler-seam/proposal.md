# Proposal: tool-options-handler-seam

## Why

Issue #357: `ToolContext` (`tool_context.h`, 399 LOC) is a 95-pure-virtual
whitelist with exactly one implementer, `ToolController`. Option state never
went through it either — `ToolController` carries ~484 LOC of storage and
setters (`tools.cpp` L187-670), ~14 `options_bar_*.cpp` files plus four panels
call `controller_->set*` directly (~94 sites), and `tools.cpp` is 1198/1200
lines, so it cannot even absorb a rename.

The state belongs with the tool family that owns the session (the crop box
already lives in `CropToolHandler`; the crop options do not), and the options
UI belongs behind a typed per-tool port instead of a 96-method god context.

## What Changes

- Option state moves into per-family **option groups** (`tool_options.h`)
  created by `ToolController` and held by the handlers that use them
  (`std::shared_ptr` for family-shared groups). A group clamps on `set` and
  notifies through callbacks the controller wires to its existing Qt signals
  and handler hooks, so every consumer keeps working.
- `ToolHandler` gains one options port: `virtual ToolOptions* options()`,
  resolved by typed `ToolController::optionsFor<T>(ToolId)`.
- `options_bar*.cpp` and the brush, clone-source, and glyphs panels write and
  read the groups directly instead of `controller_->set*` / `controller_->x()`.
- `ToolContext` is deleted. The generic services handlers use
  (`view`, `canvas`, colors, drag state, annotations, notifications) move to a
  narrow `ToolServices` interface (`tool_services.h`) with no option accessors.
- `tools.cpp` loses the option storage and setter block; the factories take the
  groups they need.

No user-visible behavior changes: the same clamps, the same signals, the same
history, and the same Qt suites.

## Capabilities

No spec delta: this is an internal seam refactor with no requirement change
(`skip_specs`).

## Impact

- New: `crates/pictura-app/cpp/tool_options.h`, `tool_services.h`.
- Deleted: `crates/pictura-app/cpp/tool_context.h`.
- `tools.{h,cpp}`, `tool_handler.h`, all `tool_*.cpp` factories/handlers,
  `options_bar*.{h,cpp}`, `panels/{brush_panel,brush_preset_picker,
  clone_source_panel,glyphs_panel,paths_panel}.cpp`, `paint_tip.h`,
  `frame_build.cpp`, tests and self-tests.
- No new dependency.

## Provenance

Independent refactor; no Adobe behavior is involved.
