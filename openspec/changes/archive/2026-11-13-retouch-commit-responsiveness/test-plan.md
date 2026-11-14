# Test plan

Every scenario is automated at the engine layer. The GUI gesture and history
contracts are unchanged and stay covered by the existing Qt suites and the
self-test.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rcr_window` | Heal a 16×16 region of a 256×256 layer with an explicit 32×32 window; record the image the operation receives and a sentinel pixel at (200, 200). | The operation receives a 32×32 image; the sentinel is unchanged. | `pictura-paint::heal_layer_materializes_only_the_requested_window` |
| `rcr_solver` | Run the Poisson and Laplace fills on a 64×64 masked rectangle and compare with the former serial relaxation. | Maximum channel difference is at most 4 units. | `pictura-paint::the_solver_matches_the_reference_poisson_relaxation`, `pictura-paint::the_solver_matches_the_reference_laplace_relaxation` |
| `rcr_solver_deterministic` | Solve the same masked rectangle twice with both fills; also run the existing Content-Aware determinism test. | Both pairs are identical. | `pictura-paint::the_relaxation_is_deterministic`, `pictura-paint::healing_is_deterministic` |
| `rcr_redeye_box` | Run Red Eye at Pupil 50 / Darken 50 over a 48×48 box on a patterned 64×64 layer; compare every pixel with the formula. | Every in-box pixel matches the formula, the returned rectangle is the box, and every outside pixel is unchanged. | `pictura-paint::red_eye_processes_a_wide_box_like_the_definition` |
| `rcr_bounds` | Build a 32×32 mask with a covered rectangle and one outlying covered pixel; call `selection_coverage`. | The bounds cover both regions and the coverage length equals the bounds' area. | `pictura-paint::selection_bounds_match_a_serial_scan` |

Manual rows: none. The release-time measurement is not a pass/fail contract;
`scripts/profile-tools.py` records it for the PR.
