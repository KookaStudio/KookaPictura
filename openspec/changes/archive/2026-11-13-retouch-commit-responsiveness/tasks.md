# Tasks

## Tests first

- [x] Add `heal_layer_materializes_only_the_requested_window` for `rcr_window`.
- [x] Add `the_solver_matches_the_reference_poisson_relaxation` and
      `the_solver_matches_the_reference_laplace_relaxation` for `rcr_solver`.
- [x] Add `the_relaxation_is_deterministic` for `rcr_solver_deterministic`.
- [x] Add `red_eye_processes_a_wide_box_like_the_definition` for
      `rcr_redeye_box`.
- [x] Add `selection_bounds_match_a_serial_scan` for `rcr_bounds`.

## Implementation

- [x] Give `heal_layer` an optional window and materialize it with
      `Plane::row_span`; write back the dirty rectangle with
      `Plane::write_rect_from`.
- [x] Compute a conservative window in `patch_layer`, `move_layer`,
      `HealStroke::commit`, and `red_eye_layer`, including the clamped clone
      source and the Proximity search reach.
- [x] Replace the serial relaxation with red-black sweeps on Rayon, keeping the
      iteration count and over-relaxation.
- [x] Process Red Eye's clipped box in parallel rows.
- [x] Find selection bounds with parallel row scans.

## Verification

- [x] `cargo fmt --all`
- [x] `cargo clippy -p pictura-paint --all-targets -- -D warnings`
- [x] `cargo test -p pictura-paint`
- [x] Run `scripts/profile-tools.py --size 17000x17000 --scenarios
      patch,spot-healing,red-eye` on an optimized build and record the release
      times in the PR.
