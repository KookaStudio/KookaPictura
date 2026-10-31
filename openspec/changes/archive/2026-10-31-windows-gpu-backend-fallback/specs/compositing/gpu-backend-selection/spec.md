# Spec Delta

## Purpose

Chooses the GPU backend a launch may use, isolates the driver probe in a child
process, and remembers per-backend outcomes so a crashing driver falls back to
the next backend or the CPU instead of taking the app down.

## ADDED Requirements

### Requirement: Backend candidates follow the platform order and the environment override

The system SHALL choose GPU backends from an explicit candidate list instead of
letting wgpu choose one. On Windows the default order SHALL be DX12, then
Vulkan. On Linux the default SHALL be Vulkan. A candidate SHALL be offered only
when its backend is compiled for the target. When `WGPU_BACKEND` is set it SHALL
replace the platform candidate order, in the order given, and SHALL NOT fall
through to a backend it does not name; `WGPU_BACKEND=none` SHALL select no GPU.
An override that names no compiled backend, or an unknown name, SHALL select no
GPU. The override SHALL NOT bypass the cache's `crashed` entries; those remain
skipped until the user clears the cache. The selection SHALL be fixed before the
first device is created.

#### Scenario: Windows defaults to DX12 then Vulkan [gb_windows_order]

- **WHEN** the selection runs on Windows with both backends compiled and no override
- **THEN** the candidates are DX12 followed by Vulkan

#### Scenario: Linux defaults to Vulkan [gb_linux_order]

- **WHEN** the selection runs on Linux with the Vulkan backend compiled
- **THEN** the only candidate is Vulkan

#### Scenario: The override selects exactly the named backend [gb_override]

- **WHEN** `WGPU_BACKEND` names Vulkan or DX12
- **THEN** the candidate list contains only that backend, even when the platform default would offer another

#### Scenario: `none` forces CPU [gb_override_none]

- **WHEN** `WGPU_BACKEND=none`
- **THEN** the candidate list is empty and compositing uses the CPU

#### Scenario: An unsupported override selects no GPU [gb_unsupported_override]

- **WHEN** `WGPU_BACKEND` names a backend that is not compiled for the target, or an unknown name
- **THEN** the candidate list is empty and the system does not fall back to another backend

#### Scenario: The default list is intersected with the compiled backends [gb_compiled_intersection]

- **WHEN** the platform default names a backend that the build did not compile
- **THEN** that backend is omitted from the candidate list

### Requirement: One backend is pinned per process

The parent process SHALL create its wgpu instance for exactly one backend, the
one selected for it. It MUST NOT pass a backend set that contains another
backend, because initializing an instance loads every named backend's driver.
The pin SHALL be resolved before the first device is created; a later attempt to
change it SHALL leave the resolved pin unchanged and MUST NOT create a second
device.

#### Scenario: A single backend is requested [gb_single_backend_pin]

- **WHEN** the parent selects DX12 or Vulkan
- **THEN** its instance is created for that backend alone and no other backend driver is initialized

#### Scenario: A late selection is refused [gb_late_pin]

- **WHEN** the backend is changed after the device has been created
- **THEN** the change is refused and the existing device keeps serving composites

### Requirement: The GPU probe runs in an isolated child process

On Windows, the system SHALL probe a candidate backend in a separate child
process before the parent creates a device for it. The child SHALL be started as
`pictura --gpu-probe <backend>`, SHALL create that backend's device, SHALL run
the existing CPU-oracle validation composite, and SHALL exit 0 on success, 1
when no adapter exists, and 2 when a device exists but the validation fails. The
parent SHALL treat a child that exits 0 as `ok`, exits 1 as `failed`, and exits
2, is terminated by a signal or exception, or exceeds a 10 s timeout, as
`crashed`. A crashed or timed-out child MUST NOT terminate the
parent. On Windows the child SHALL suppress the WER/JIT dialogs and terminate on
an unhandled exception. A backend that is not compiled into the build SHALL be
rejected without spawning a child. The parent MUST NOT create the device before
the child exits.

#### Scenario: A successful probe selects the backend [gb_probe_ok]

- **WHEN** the child exits 0
- **THEN** the backend is recorded as `ok` and selected for the parent

#### Scenario: A clean failure moves to the next candidate [gb_probe_clean_failure]

- **WHEN** the child exits 1 because no adapter exists
- **THEN** the backend is recorded as `failed` and the next candidate is tried

#### Scenario: A child crash does not kill the parent [gb_probe_crash]

- **WHEN** the child is terminated by a signal or an unhandled exception, or exits 2 after a failed validation
- **THEN** the backend is recorded as `crashed`, the parent continues, and the next candidate is tried

#### Scenario: A hung child is killed [gb_probe_timeout]

- **WHEN** the child does not finish within 10 s
- **THEN** the parent kills it, records `crashed`, and continues

#### Scenario: An uncompiled backend is rejected without a child [gb_probe_uncompiled]

- **WHEN** the requested backend is not compiled for the target
- **THEN** the probe returns a clean failure and spawns no process

#### Scenario: The probe does not create the parent device [gb_probe_before_parent]

- **WHEN** the probe runs
- **THEN** the parent's process-wide device is still uncreated, so the parent's own init is the only place that can crash

### Requirement: The backend cache records the last outcome per backend

On Windows, the system SHALL persist one outcome per backend in
`kooka-pictura/gpu-backend.json` next to `state.json`: `ok`, `failed`,
`crashed`, or `unknown`. A cache written by a different application version
SHALL be read as all `unknown`. An `ok` backend SHALL be selected without a
probe. A `failed` backend SHALL be probed again on the next launch. A `crashed`
backend SHALL NOT be probed again until the user re-enables GPU compute or the
cache is cleared. Re-enabling GPU compute SHALL clear the cache. A missing,
unreadable, or malformed cache SHALL read as all `unknown` and MUST NOT fail the
launch.

#### Scenario: Outcomes round-trip [gb_cache_roundtrip]

- **WHEN** a cache with `ok`, `failed`, and `crashed` entries is saved and loaded
- **THEN** each backend loads with the same outcome

#### Scenario: A version change invalidates the cache [gb_cache_version]

- **WHEN** a cache written by a different application version is loaded
- **THEN** every backend reads as `unknown`

#### Scenario: `ok` skips the probe [gb_cache_ok_skips_probe]

- **WHEN** a backend is cached as `ok` for the current version
- **THEN** it is selected without spawning a probe

#### Scenario: `failed` is retried [gb_cache_failed_retry]

- **WHEN** a backend is cached as `failed`
- **THEN** the next launch probes it again

#### Scenario: `crashed` is skipped until reset [gb_cache_crashed_skip]

- **WHEN** a backend is cached as `crashed` and the user has not reset the cache
- **THEN** the next launch does not probe it

#### Scenario: Re-enabling GPU compute clears the cache [gb_cache_reset]

- **WHEN** the user turns Use GPU Compute off and back on
- **THEN** the cache is cleared and the next launch probes the candidates again

#### Scenario: A corrupt cache is unknown [gb_cache_corrupt]

- **WHEN** the cache file is missing, unreadable, or malformed
- **THEN** every backend reads as `unknown` and the launch continues

### Requirement: A crash marker turns an in-process init crash into a safe-mode fallback

On Windows, before the parent creates a device for the selected backend it SHALL
write `gpu-init.marker` next to `state.json`, naming that backend, the
application version, and the writer's PID, and it SHALL remove the marker after
the device probe returns. On startup, a marker left by a previous launch of the
same version SHALL mark the named backend `crashed`, SHALL be removed, and SHALL
cause the next candidate to be selected. A marker whose recorded PID is a live
process SHALL be ignored and SHALL NOT change the cache, so a concurrent
instance is not mistaken for a crash. A clean launch MUST NOT leave a marker
behind. A marker naming an unknown backend, or written by another application
version, SHALL be removed without changing the cache.

#### Scenario: A successful init clears the marker [gb_marker_clear]

- **WHEN** the parent's device probe returns normally
- **THEN** no marker remains

#### Scenario: A stale marker marks the backend crashed [gb_marker_crash]

- **WHEN** the app starts and a marker from a previous launch of the same version is present
- **THEN** the named backend is recorded as `crashed` and the marker is removed

#### Scenario: A stale marker moves to the next candidate [gb_marker_next]

- **WHEN** the marker names the first candidate and another candidate exists
- **THEN** the next candidate is selected

#### Scenario: An unknown marker is removed without a cache change [gb_marker_unknown]

- **WHEN** the marker names a backend the build does not know, or was written by another version
- **THEN** the marker is removed and the cache is unchanged

### Requirement: Linux keeps the direct Vulkan path

On Linux the system SHALL keep the existing direct Vulkan device path: no child
probe, no backend cache, and no crash marker. `gpu_available()` SHALL retain its
cached, never-panicking contract. `WGPU_BACKEND=none` SHALL still force the CPU.

#### Scenario: Linux skips the Windows chain [gb_linux_direct]

- **WHEN** the app starts on Linux
- **THEN** it creates the Vulkan device directly, writes no cache or marker, and does not spawn `--gpu-probe`

#### Scenario: Linux honors `none` [gb_linux_none]

- **WHEN** `WGPU_BACKEND=none` is set on Linux
- **THEN** compositing uses the CPU

### Requirement: The zero-copy interop probe remains Vulkan-only

`--interop-probe` SHALL create a Vulkan device regardless of the selected
compositing backend and SHALL NOT use DX12. When Vulkan is unavailable it SHALL
report no device and exit without panicking. The interop path MUST NOT be
selected by the backend chain.

#### Scenario: Interop requests Vulkan even when DX12 is selected [gb_interop_vulkan]

- **WHEN** `--interop-probe` runs on Windows regardless of the configured compositing backend
- **THEN** it creates a Vulkan device and does not use DX12

#### Scenario: Interop fails cleanly without Vulkan [gb_interop_unavailable]

- **WHEN** `--interop-probe` runs where Vulkan is unavailable
- **THEN** it reports no device and exits without panicking
