# Tasks: healing-brush-tools

## 1. Engine

- [x] 1.1 `pictura_paint::healing` — `RgbaImage`, `heal_region` (three types), `clone_region`, `heal_layer`; unit tests.
- [x] 1.2 `HealStroke` coverage gesture; unit tests.

## 2. Bridge

- [x] 2.1 `cxxqt_object/healing.rs`; `PictureViewRust::heal_stroke`.

## 3. Tools

- [x] 3.1 `tool_healing.cpp` (Spot Healing + Healing); catalog rows enabled; options-bar rows.
- [x] 3.2 J-group toolbox flyout cycles the two tools.

## 4. Verification

- [x] 4.1 `healing_tools` self-test (536); `shift_plain` (117) asserts the J cycle; `bash scripts/verify-fast.sh`.
