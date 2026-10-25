# Test plan

The Rust tests run under nextest. Device-dependent tests are `#[ignore]`d and run
in the `gpu-parity` CI job
(`--features software-vulkan --profile gpu --run-ignored all`).

## compositing/gpu-compositing

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Every declining content route has a reason | For each declining content route, build a one-layer document and call `check_supported`. | Each returns its `GpuError`; no device is needed. | `pictura-render::every_declining_route_names_its_reason` |
| Effects win over the content route | Existing coverage: a `SoCo` fill layer with an enabled drop shadow. | `UnsupportedLayerEffect`. | `pictura-render::gpu_reports_the_effect_for_a_fill_layer` |
| The CPU oracle is unchanged | Classify a corpus covering every content route with `content_route` and with a verbatim copy of the old if/else chain; run the existing oracle suites. | The classifications agree, so the unchanged render steps produce identical bytes; the oracle suites pass. | `pictura-render::content_route_matches_the_legacy_chain`, `pictura-render::solid_scene_matches_imagemagick`, `pictura-render::ramp_scene_matches_imagemagick` |
| A 16-bit document declines | A document with `source_depth = Some(Sixteen)`. | `UnsupportedDepth`. | `pictura-render::sixteen_bit_document_declines` |
| The active backend falls back for a 32-bit document | `composite_active(doc, true)` on a 32-bit document. | `Backend::Cpu`, and bytes equal to `composite_rgba`. | `pictura-render::thirty_two_bit_active_backend_is_cpu` |
| An 8-bit document stays eligible | `check_supported` on plain 8-bit layers. | Ok. | `pictura-render::eight_bit_document_stays_eligible` |
| An enabled Camera Raw filter declines | A pixel smart object with an enabled Camera Raw filter. | `UnsupportedSmartFilter`. | `pictura-render::enabled_smart_filter_declines` |
| A disabled filter group still declines | The same layer with the master switch off. | `UnsupportedSmartFilter`. | `pictura-render::disabled_filter_group_still_declines` |
| Disabled filters stay eligible | Every per-filter flag off. | Ok. | `pictura-render::disabled_smart_filters_stay_eligible` |
| A device that matches the oracle stays enabled | Under software-vulkan, `gpu_available()` and `composite_active` on the validation scene. | true and `Backend::Gpu`. | `pictura-render::gpu_is_available_under_software_vulkan` |
| A device that diverges is treated as absent | Existing coverage: the gate agrees with the validation comparison. | Consistent. | `pictura-render::the_gpu_gate_is_consistent_with_the_validation_scene` |
| The GPU paint stroke falls back on a diverging device | Existing coverage. | The stroke gate follows the validation. | `pictura-render::the_stroke_gate_follows_the_compositor_validation` |
| The CPU-only mode check still precedes the gate | Existing coverage, still unignored. | `UnsupportedMode(Dissolve)`. | `pictura-render::dissolve_still_cpu_only` |
| The check runs at most once | Call `gpu_available()` twice and compare the validation-run counter. | It increases by at most 1. | `pictura-render::validation_runs_at_most_once` |

## verification/gpu-parity-ci

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Production rejects a software device | Without the feature: `accelerates(DeviceType::Cpu)`. | false. | `pictura-render::a_software_adapter_is_no_gpu` |
| The test feature accepts a software device | With the feature: `accelerates(DeviceType::Cpu)`. | true. | `pictura-render::software_vulkan_accepts_a_cpu_device` |
| The application build leaves the feature off | In `pictura_app` tests, read `pictura_render::SOFTWARE_VULKAN`. | false. | `pictura-app::app_never_enables_software_vulkan` |
| A default run reports GPU tests as skipped | Run `scripts/test-report.sh` and read the summary. | The GPU parity tests are in SKIP. | manual: needs a full report run; the PR body quotes the skip counts before and after |
| A run without a device fails | Call the device-requirement helper (`require_device`, which `require_gpu` wraps) with no device. | It panics with the device message. | `pictura-render::require_gpu_panics_without_a_device` |
| Device-free GPU checks always run | Default run. | The decline tests pass. | `pictura-render::every_declining_route_names_its_reason` |
| The job exercises the GPU | The gpu job. | `gpu_available()` is true, and the parity tests pass. | `pictura-render::gpu_is_available_under_software_vulkan` |
| A broken driver setup fails the job | Push a throwaway commit without `mesa-vulkan-drivers`. | The job fails. | manual: needs a CI run with a deliberately broken runner image |
| A GPU-eligible scene matches the oracle | Every fixture scene through `composite_active(doc, true)`. | Gpu and ±1 LSB, or Cpu and equal. | `pictura-render::fixture_scenes_match_oracle_on_active_backend` |
| A declined scene equals the oracle | A fixture scene set to Dissolve through the active backend. | Cpu, byte-equal. | `pictura-render::declined_fixture_scene_equals_oracle` |
