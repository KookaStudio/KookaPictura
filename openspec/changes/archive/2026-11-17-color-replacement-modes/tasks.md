# Tasks

## 1. Tests first

- [x] 1.1 Add the `pictura-paint` tests named in `test-plan.md`: mode round
  trip, Normal/Multiply, Behind/Clear, 32-bit subset.
- [x] 1.2 Add the `pictura-app` bridge test for the flat-mode refusal.
- [x] 1.3 Add `tst_color_replacement` with the menu, 32-bit, bitmap, indexed,
  Multichannel, RGB, and repaint cases, and register it in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 2. Engine

- [x] 2.1 Replace `ReplaceMode` with the 25-mode CS6 paint list, its names, and
  `ALL`/`from_i32`.
- [x] 2.2 Dispatch each mode through `pictura_core::blend::blend`; make `Behind`
  a no-op and `Clear` erase alpha.
- [x] 2.3 Add the 32-bit subset predicate.

## 3. Bridge

- [x] 3.1 Add `color_replacement_available`, `color_replacement_mode_count`,
  `color_replacement_mode_name`, and `color_replacement_mode_allowed`.
- [x] 3.2 Gate `begin_color_replacement` on the document's effective colour
  mode.

## 4. Shell

- [x] 4.1 Add `toolAllowedForDocument` and use it in `setActiveTool`,
  `bindCanvas`, and the toolbox.
- [x] 4.2 Add the toolbox availability refresh and the Move fallback.
- [x] 4.3 Build the Color Replacement Mode combo from the bridge and rebuild it
  for 32-bit documents.
- [x] 4.4 Refuse with a mode message in `tool_colorreplacement.cpp`.
- [x] 4.5 Update the existing self-test check 540's mode indices.

## 5. Close-out

- [x] 5.1 Tick #547 in `ROADMAP.md`.
- [x] 5.2 Run `openspec validate --all --strict`, the targeted Rust/Qt tests,
  fmt, and clippy.
- [x] 5.3 Archive the change with `openspec archive color-replacement-modes`.
