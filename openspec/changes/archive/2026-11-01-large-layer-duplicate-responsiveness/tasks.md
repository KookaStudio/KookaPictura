# Tasks

- [x] 1.1 Add duplicate, move and delete scenarios and RSS to `scripts/profile-tools.py`; record the baseline
- [x] 1.2 Make `Bands::diverged` O(1); verify with `cargo nextest run -p pictura-core plane`
- [x] 1.3 Share a parallel row scan between `coverage_bounds` and `plane_coverage_bounds`, and use it in `trim_bounds` and `content_bounds`; verify with `cargo nextest run -p pictura-render content_rect`
- [x] 1.4 Add `duplicate_is_content_neutral` and use it at both duplicate sites; verify with `cargo nextest run -p pictura-render neutral`
- [x] 1.5 Fill the released Background's alpha plane in parallel with `Plane::build`
- [x] 1.6 Warm the Move preview only on a real switch to the tool; verify with `tst_move_feedback`
- [x] 1.7 Re-run the profile on the real file and compare with the baseline
