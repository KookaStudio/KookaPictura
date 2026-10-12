# Tasks: tool-options-handler-seam

## 1. Headers

- [x] 1.1 `tool_options.h`: option structs (moved), `ToolOptions` base,
  `ToolOption<T>`, and the explicit Paint / Selection / Crop / CloneSource
  groups with their clamping and callbacks.
- [x] 1.2 `tool_services.h`: the generic `ToolServices` interface.
- [x] 1.3 `tool_handler.h`: `ToolServices&` parameters + the
  `optionGroups()` / `addOptions()` port.

## 2. Controller

- [x] 2.1 `tools.h/.cpp`: ToolController implements `ToolServices`; option
  storage and the ~484-line setter block deleted; groups created, wired, and
  passed to factories; `optionsFor<T>` resolver.
- [x] 2.2 Factories and handlers updated to their groups; `paint_tip.h` takes
  the paint group.

## 3. UI

- [x] 3.1 `options_bar*.{h,cpp}` retargeted onto the groups.
- [x] 3.2 brush, brush-preset, clone-source, glyphs, and paths panels
  retargeted.
- [x] 3.3 `tool_context.h` deleted; includes retargeted.

## 4. Tests

- [x] 4.1 Qt tests and self-tests retargeted mechanically.
- [x] 4.2 `ctest -R '^tst_'` (78/78), the app self-test (474/0/0),
  `check-file-size.sh`, `check-selftest-budget.sh`, `guard.sh`,
  `openspec validate --all --strict`, `bash scripts/verify-fast.sh` (the only
  failure is the pre-existing local ImageMagick fixture mismatch in
  `pictura-render`).
