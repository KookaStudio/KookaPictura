# Tasks: brush-popup-solo-mask

## 1. Measurement

- [x] 1.1 Probe the reporter's flow (solo on, paint a stroke, open the picker, hover) on a 4000×3000 mask document with `PICTURA_PAINT_TIMING=1`.
- [x] 1.2 Attribute the stall: the debounced `refreshPanels` costs 168–178 ms, its bulk `channels_active_mask_thumbnail(24)` at ~170 ms.

## 2. Tests

- [x] 2.1 `pictura-app::mask_thumbnail_samples_touch_only_the_grid`.
- [x] 2.2 `pictura-app::mask_thumbnail_samples_match_the_full_plane_downsample`.

## 3. Fix

- [x] 3.1 Extract `mask_thumbnail_samples` and make `mask_channel_thumbnail` sample the thumbnail grid directly, leaving the pixels unchanged.

## 4. Verification

- [x] 4.1 `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, the Qt suites, `openspec validate --all --strict`, `scripts/verify-fast.sh` (the only failure is the pre-existing local `ramp_Divide.rgba` ImageMagick fixture mismatch, unrelated to this change).
- [x] 4.2 Update `docs/dev/STATE.md` (docs change; carries `TASK-ALLOWS-DOCS`). No `ROADMAP.md` change: no feature shipped or removed.
- [x] 4.3 Archive the change into `openspec/specs/ui/channels-panel` in this pull request.
