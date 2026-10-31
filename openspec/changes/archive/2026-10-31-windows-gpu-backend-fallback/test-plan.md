# Test Plan

One row per scenario in the four delta specs. GPU device tests are `#[ignore]`d
and run in the `gpu-parity` job on lavapipe; the Windows DX12 probe runs in the new
`windows-render` CI job. Manual rows are listed in the PR body as a reviewer
checklist: the two self-test rows, the probe-before-parent row, the four hardware
rows on the reporter's i5-6500 (DX12 with no `igvk64.dll`, forced Vulkan child
crash surviving on CPU, a hung probe killed after 10 s, no WER/JIT dialog), the
two diverging-device rows, and the no-Vulkan interop row.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `Offscreen render is displayed` | Run the offscreen gradient render with a usable adapter on the selected backend | The readback is wrapped as a QImage and shown | `pictura-app::offscreen_gradient_is_non_blank` |
| `Self-test passes headless` | Run `./build/pictura --headless --self-test` under the offscreen platform | It reports a non-blank composite and exits 0 | manual: the self-test is an executable oracle, not a Rust unit test; the Linux `qt-headless` CI job and verify-full run it, and `windows.yml` runs it only on release PRs/tags |
| `Self-test without an adapter` | Run `--self-test` on a host with no adapter on the selected backend | The offscreen path reports unavailability, shows the CPU image, and exits 0 | manual: needs a host with no usable adapter; the Linux `qt-headless` runner has no usable GPU (the app never enables `software-vulkan`), and the GPU-less `windows.yml` runner runs the self-test only on release PRs/tags; `windows-render` does not build the app |
| `gb_windows_order` | Resolve candidates on Windows with DX12 and Vulkan compiled, no override | DX12 then Vulkan | `pictura-render::windows_default_candidates_are_dx12_then_vulkan` |
| `gb_linux_order` | Resolve candidates on Linux with Vulkan compiled | Only Vulkan | `pictura-render::linux_default_candidates_are_vulkan_only` |
| `gb_override` | Set `WGPU_BACKEND=vulkan` and `=dx12` | Only the named backend is selected | `pictura-render::override_selects_exactly_one_backend` |
| `gb_override_none` | Set `WGPU_BACKEND=none` | Empty candidate list; CPU compositing | `pictura-render::none_override_selects_no_backend` |
| `gb_unsupported_override` | Set `WGPU_BACKEND=metal` on Linux, or an unknown name | Empty candidate list, no fallback | `pictura-render::unsupported_override_selects_no_backend` |
| `gb_compiled_intersection` | Resolve a default list that names an uncompiled backend | The uncompiled backend is omitted | `pictura-render::candidates_intersect_compiled_backends` |
| `gb_single_backend_pin` | Pin DX12 or Vulkan and inspect the requested wgpu bitmask | Exactly one backend bit is requested | `pictura-render::single_backend_pin_is_one_wgpu_bit` |
| `gb_late_pin` | Pin a compiled backend, create the device, then call `set_backend` with another backend | The change is refused and the existing device stays | `pictura-render::late_pin_is_refused_after_resolution` |
| `gb_probe_ok` | Run the child probe on a working Vulkan device | Exit 0; the backend is selected | `tst_gpu_backend::selectGpuBackendRecordsOkAndSelectsIt`, `pictura-render::vulkan_probe_is_clean_on_a_software_device` |
| `gb_probe_clean_failure` | Classify a child exit code of 1 | Recorded as `failed`, next candidate tried | `tst_gpu_backend::selectGpuBackendRecordsFailedAndTriesNext` |
| `gb_probe_crash` | Classify a child terminated by a signal or an access violation (`0xC0000005`) | Recorded as `crashed`, the parent continues | `tst_gpu_backend::selectGpuBackendRecordsCrashAndTriesNext` |
| `gb_probe_timeout` | Spawn a child that sleeps past the 10 s deadline | Killed, recorded as `crashed`, parent returns | `tst_gpu_backend::selectGpuBackendTreatsTimeoutAsCrashed`, `pictura-app::wait_with_timeout_kills_a_hung_child` |
| `gb_probe_uncompiled` | Probe a backend the build did not compile | Clean failure, no child spawned | `pictura-render::probe_backend_rejects_an_uncompiled_backend` |
| `gb_probe_before_parent` | Resolve candidates and read the pin without calling `gpu_available()` | The parent's device is still uncreated | manual: the parent never calls `gpu_available()` before the frame; the child is a separate process |
| `gb_cache_roundtrip` | Save and load `ok`, `failed`, and `crashed` entries | Each backend loads the same outcome | `tst_gpu_backend::okRoundTrips`, `tst_gpu_backend::failedRoundTrips`, `tst_gpu_backend::crashedRoundTrips` |
| `gb_cache_version` | Load a cache written by another app version, then one with a future `schemaVersion` | Every backend reads `unknown` | `tst_gpu_backend::versionBumpInvalidatesEverything`, `tst_gpu_backend::futureSchemaVersionLoadsUnknown` |
| `gb_cache_ok_skips_probe` | Cache `ok` for the current version and select | No probe is spawned | `tst_gpu_backend::okBackendSkipsTheProbe` |
| `gb_cache_failed_retry` | Cache `failed` and launch again | The backend is probed again | `tst_gpu_backend::failedIsRetriedNextLaunch` |
| `gb_cache_crashed_skip` | Cache `crashed` and launch again | The backend is not probed | `tst_gpu_backend::crashedIsSkippedUntilReset` |
| `gb_cache_reset` | Open Preferences > Performance and toggle Use GPU Compute off then on | The cache is deleted and the next launch probes | `tst_gpu_backend::preferenceReenableClearsCache` |
| `gb_cache_corrupt` | Write malformed JSON, then delete the file | Both read as all `unknown`; the launch continues | `tst_gpu_backend::corruptCacheLoadsUnknown`, `tst_gpu_backend::missingFileLoadsAllUnknown`, `tst_gpu_backend::corruptCacheDoesNotFailTheLaunch` |
| `gb_marker_clear` | Complete a successful parent device probe | The marker file is gone | `tst_gpu_backend::successClearsTheMarker` |
| `gb_marker_crash` | Pre-create a same-version marker, then start | The named backend is `crashed`; the marker is removed | `tst_gpu_backend::staleMarkerMarksTheBackendCrashed` |
| `gb_marker_next` | Pre-create a marker for the first candidate | The next candidate is selected | `tst_gpu_backend::staleMarkerFallsToTheNextCandidate` |
| `gb_marker_unknown` | Pre-create a marker with an unknown backend, another version, or malformed JSON | The marker is removed; the cache is unchanged | `tst_gpu_backend::unknownMarkerIsRemovedWithoutACacheChange`, `tst_gpu_backend::oldVersionMarkerIsIgnored`, `tst_gpu_backend::malformedMarkerIsRemoved` |
| `gb_linux_direct` | Start on Linux and inspect the state dir | Vulkan is created directly; no cache, marker, or child | `tst_gpu_backend::linuxSkipsTheProbeAndKeepsVulkan` |
| `gb_linux_none` | Set `WGPU_BACKEND=none` on Linux | CPU compositing | `pictura-render::none_override_selects_no_backend` |
| `gb_interop_vulkan` | Run `--interop-probe` on Windows regardless of the configured compositing backend | A Vulkan device is created; DX12 is not used | manual: needs Windows DX12 hardware; the code forces Vulkan |
| `gb_interop_unavailable` | Run `--interop-probe` where Vulkan is unavailable | Reports no device, exits without panicking | manual: needs a host with no Vulkan; lavapipe always provides one |
| `No Vulkan adapter` | Composite with no usable adapter on the selected backend | `Err(Unavailable)`; `composite_gpu_or_cpu` returns the CPU composite | `pictura-render::cpu_pin_composites_on_cpu` |
| `Document exceeds device limits` | Composite a document past the device buffer or dispatch limits | `Err(TooLarge)`, no panic | `pictura-render::document_past_2d_product_limit_returns_too_large` |
| `Fallback for a CPU-only stack` | Pass a CPU-only stack to `composite_gpu_or_cpu` | The buffer equals `composite_rgba` byte for byte | `pictura-render::fallback_matches_cpu_for_cpu_only_mode` |
| `A device that matches the oracle stays enabled` | Composite the validation scene on a matching device | `gpu_available()` true; `Backend::Gpu` | `pictura-render::the_gpu_gate_is_consistent_with_the_validation_scene` |
| `A device that diverges is treated as absent` | Composite the validation scene on a diverging device | `gpu_available()` false; CPU path; `shared_device()` None | manual: needs a device that accepts pipelines but returns wrong pixels (observed on Mesa Honeykrisp) |
| `The GPU paint stroke falls back on a diverging device` | Seed a GPU stroke after validation fails | `GpuStroke::available()` false; `new` returns `Unavailable` | manual: needs a device that accepts pipelines but returns wrong pixels (observed on Mesa Honeykrisp) |
| `The CPU-only mode check still precedes the gate` | Composite a Dissolve stack on a device that fails validation | `UnsupportedMode(Dissolve)`, not `Unavailable` | `pictura-render::cpu_only_modes_error_without_panicking` |
| `The check runs at most once` | Call `gpu_available()` repeatedly | The validation scene runs at most once | `pictura-render::validation_runs_at_most_once` |
| `Adapter present and GPU enabled` | Composite with GPU enabled on a usable selected adapter | `Backend::Gpu`, within ±1 LSB | `pictura-render::enabled_gpu_matches_availability` |
| `GPU disabled or unavailable` | Composite with GPU disabled, or with no adapter | `Backend::Cpu`, byte-equal to `composite_rgba` | `pictura-render::disabled_gpu_returns_cpu_backend` |
| `Probe reports a usable adapter` | Call `gpu_available()` on a working selected adapter | True; repeated calls cached | `pictura-render::the_gpu_gate_is_consistent_with_the_validation_scene` |
| `No candidate is selected` | Select with `WGPU_BACKEND=none` or an empty candidate list | `gpu_available()` false, no panic | `pictura-render::none_override_selects_no_backend`, `pictura-render::cpu_pin_composites_on_cpu` |
| `Probe reports no adapter` | Call `gpu_available()` with no usable adapter | False, no panic | `pictura-render::cpu_pin_composites_on_cpu` |
| `Backend indicator values` | Compute the label for DX12, Vulkan, user-disabled, no adapter, and blocked | The five expected labels | `pictura-render::backend_label_names_all_five_states`, `tst_gpu_backend::statusBarShowsTheBackendLabel`, `tst_gpu_backend::aboutTextNamesBackend` |
| `gb_label_dx12` | DX12 active | `GPU (DX12)` | `pictura-render::backend_label_names_all_five_states` |
| `gb_label_vulkan` | Vulkan active | `GPU (Vulkan)` | `pictura-render::backend_label_names_all_five_states` |
| `gb_label_cpu` | Usable adapter, compute off | `CPU` | `pictura-render::backend_label_names_all_five_states` |
| `gb_label_no_gpu` | No usable adapter, nothing blocked | `CPU (no GPU)` | `pictura-render::backend_label_names_all_five_states` |
| `gb_label_safe_mode` | Adapter found but the chain is blocked | `CPU (safe mode)` | `pictura-render::backend_label_names_all_five_states` |
| `Document opens without a GPU` | Open a document with no usable candidate | CPU composite, no panic or dialog | `pictura-render::cpu_pin_composites_on_cpu` |
| `Toggle stays disabled without an adapter` | Offer the GPU-compute command with no backend | Disabled; invoking it does not enable the GPU | `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` |
| `gb_safe_mode_toggle` | In safe mode, toggle Use GPU Compute off and on | The toggle is available and the cache is cleared | `tst_gpu_backend::preferenceReenableClearsCache`, `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` |
| `Manual: DX12 with no igvk64.dll` | Launch on the reporter's i5-6500 / HD 530 | DX12 is selected and `igvk64.dll` is not loaded in the app process | manual: needs the reported Intel Gen9 hardware |
| `Manual: forced Vulkan child crash` | Force the Vulkan probe on the same machine | The child crashes, the parent survives and falls back to CPU | manual: needs the reported Intel Gen9 hardware and the crashing ICD |
| `Manual: hung probe killed` | Force a probe that hangs past 10 s | The parent kills it after 10 s and continues on the next candidate or CPU | manual: needs a controllable hanging Vulkan driver |
| `Manual: no WER/JIT dialog` | Repeat the forced Vulkan probe | No Windows Error Reporting or Just-In-Time debugger dialog appears | manual: needs Windows and the reported Intel Gen9 hardware |
