# Tasks: paintbrush-reference-texture

## 1. Tests

- [x] 1.1 Extend `pictura-paint::grain_breaks_up_the_dab_alpha` and add
  `pictura-paint::grain_strength_matches_the_pencil_presets`.
- [x] 1.2 Update `tst_brush_tip_preview` for hardness 65 and grain 50 / 75.

## 2. Texture

- [x] 2.1 Mix the per-pixel hash with 2 px and 3 px value noise in
  `fill::grain_noise`.
- [x] 2.2 Set both pencil presets to hardness 65; `2B Pencil` grain 50 and
  `Non-Photo Blue Pencil` grain 75.

## 3. Verification

- [x] 3.1 Run `cargo nextest run -p pictura-paint`, the Qt suite, and the
  verify-fast gates. The sccache absent-case check fails in this environment
  because CMake still finds `/usr/bin/sccache` through its system prefix path;
  every other gate passed.
- [x] 3.2 Update `docs/dev/STATE.md` and archive the change.
