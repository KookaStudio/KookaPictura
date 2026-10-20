# Tasks: pencil-grain-texture

## 1. Tests

- [x] 1.1 Add `pictura-paint::grain_noise_is_deterministic_and_clumpy`.
- [x] 1.2 Extend `pictura-paint::grain_breaks_up_the_dab_alpha` with the
  coverage-range assertion.
- [x] 1.3 Update `tst_brush_tip_preview` for the 2B Pencil grain 80.

## 2. Grain

- [x] 2.1 Add `fill::grain_noise` (fine + 2 px smooth noise).
- [x] 2.2 Use `grain_noise` in `Stroke::sample`'s grain modulation.
- [x] 2.3 Raise the `2B Pencil` preset's grain to 80.

## 3. Verification

- [x] 3.1 Run `cargo nextest run -p pictura-paint`, the Qt suite, and
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`.
- [x] 3.2 Update `docs/dev/STATE.md` and archive the change.
