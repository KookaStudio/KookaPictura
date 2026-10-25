# Tasks

No build or test runs during implementation (AGENTS.md workflow step 4). The
checks run once, in group 4. This change builds on
`render-performance-instrumentation` (`pictura_render::metrics`).

## 1. Tests first

- [x] 1.1 Write the eligibility tests in `gpu/eligibility_tests.rs` (every declining route, depth, smart-filter cases, 8-bit eligible; effect precedence keeps its existing test), `content_route_matches_the_legacy_chain` (a verbatim copy of the old if/else chain as the oracle), `validation_runs_at_most_once`, `software_vulkan_accepts_a_cpu_device`, `require_gpu_panics_without_a_device`, and `gpu_is_available_under_software_vulkan`
- [x] 1.2 Write `app_never_enables_software_vulkan` in `pictura_app`
- [x] 1.3 Add `fixture_scenes_match_oracle_on_active_backend` and `declined_fixture_scene_equals_oracle` (ignored) to `tests/oracle.rs`

## 2. Content route classification (#745)

- [x] 2.1 Add `composite_route.rs` (`ContentRoute`, `content_route`, `smart_filters_apply`) and dispatch `composite_layer_inner` through an exhaustive match that keeps the existing fall-through
- [x] 2.2 Add `GpuError::UnsupportedDepth` and `UnsupportedSmartFilter`, rewrite `check_supported` as the depth gate plus the ordered per-layer checks over an exhaustive match, and count declines

## 3. Software device and honest skips (#746)

- [x] 3.1 Add the `software-vulkan` feature and `SOFTWARE_VULKAN`, and gate `accelerates()` and its test
- [x] 3.2 Add the `require_gpu` helpers; ignore and require in every device-dependent test listed in design D5; split the fresh-white CPU arm
- [x] 3.3 Add the `gpu` nextest profile and the `gpu-parity` CI job (clippy plus nextest with the feature)
- [x] 3.4 Document the gpu profile, the feature, and the local lavapipe command in `DEVELOPING.md`

## 4. Verification (run once, after all code is written)

- [ ] 4.1 `cargo fmt`, clippy (default and with `software-vulkan`), `cargo nextest run -p pictura-render -p pictura_app`, and `check-test-plan.py`; a local lavapipe run of the gpu profile; quote the report's skip counts before and after in the PR body
- [ ] 4.2 Confirm the `gpu-parity` job is green on the PR, and run the manual broken-driver check
