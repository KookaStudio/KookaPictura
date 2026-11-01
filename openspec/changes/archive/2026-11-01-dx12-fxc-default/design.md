# Design

## Context

See `proposal.md` — Why. The constraints that shape the implementation:

- Two independent device-creation sites build the instance with
  `InstanceDescriptor::new_without_display_handle()`:
  `crates/pictura-render/src/gpu/backend.rs:246` (compositor and probe) and
  `crates/pictura-app/src/gpu.rs:70` (offscreen gradient render). Neither reads
  `WGPU_DX12_COMPILER`.
- wgpu 30's `Dx12Compiler::Auto` tries static DXC, then `dxcompiler.dll`, and
  falls back to FXC only on a *load* failure
  (`wgpu-hal-30.0.1/src/dx12/instance.rs:64-85`). A loadable but too-old DXC
  fails instance creation or pipeline compilation instead.
- The probe runs in a child process; only its exit code crosses back
  (`crates/pictura-app/src/gpu_probe_bridge/imp.rs:8-46`). The cache entry
  already has an optional `detail` field that is loaded (`gpu_probe.cpp:194`)
  and saved (`:209-210`) but never written; `selectGpuBackend`
  (`crates/pictura-app/cpp/gpu_probe.cpp:329-350`) writes only `state` and
  `updatedAt`.
- `accelerates()` rejects CPU-type devices unless `SOFTWARE_VULKAN` is set
  (`crates/pictura-render/src/gpu/backend.rs:306-308`). WARP is reported as
  `DeviceType::Cpu`, so the Windows runner currently proves only "no adapter".
- The `software-vulkan` feature is a test-only switch; `windows-render`
  (`ci.yml:249`) has no Qt, and its step at `ci.yml:280` runs
  `cargo test -p pictura-render --lib --test backend_pin --test backend_cpu`.

## Goals / Non-Goals

**Goals:**

- A Windows DX12 launch uses FXC deterministically, with no reliance on the
  deployed DXC version, and still honors `WGPU_DX12_COMPILER`.
- A failed or successful probe leaves a human-readable reason in
  `gpu-backend.json`.
- Windows CI compiles the real compositor and filter shaders with FXC on WARP,
  so a compiler regression fails the job instead of reaching a user.
- No change to the candidate order, pin, crash marker, or cache policy.

**Non-Goals:**

- Shipping a modern DXC (>= 1.8.2502). It costs ~15 MB and no current shader
  needs shader model 6.
- Version-gating the deployed DXC at runtime.
- Making `WGPU_BACKEND` bypass the cache. The existing reset path is enough for
  manual diagnosis.
- Changing the Windows package contents.

## Decisions

### D1. One compiler policy, called from both instance sites

`pictura-render` gains `dx12_compiler() -> wgpu::Dx12Compiler`, implemented as
`resolve_dx12_compiler(Dx12Compiler::from_env())` where the inner function
returns `env.unwrap_or(Dx12Compiler::Fxc)`. The split keeps the policy testable
without mutating process-global environment variables, which would race other
tests. `dx12_compiler_label()` names the configured variant `FXC`, `DXC`,
`STATICDXC`, or `AUTO`; `Auto` stays `AUTO` rather than claiming FXC, because
wgpu may still pick DXC for it. The policy tests use `matches!` because
`wgpu::Dx12Compiler` derives no `PartialEq`. Both instance sites set
`descriptor.backend_options.dx12.shader_compiler = dx12_compiler()`.

`new_without_display_handle()` is kept instead of switching to `with_env()`:
`with_env()` also lets `WGPU_BACKEND` and instance flags override the pinned
backend, which would defeat the one-backend-per-process rule.

Alternative rejected: remove the deployed DXC and keep `Auto`. It is implicit,
depends on the DLL search order, and a stale system DXC would still fail.
Alternative rejected: probe the DXC version before choosing. It needs
Windows-specific FFI in `pictura-render` and duplicates wgpu's loader logic for
a one-time startup decision.

### D2. The detail crosses the child boundary on stdout

`ProbeOutcome` stays the classification; a new `ProbeReport { outcome, detail }`
carries the text. `gpu_probe_child` caps the detail at 512 characters, prints it as
one line, and exits with the mapped code. The parent spawns the child with
`Stdio::piped()` for stdout, takes the pipe before waiting, and reads it only
after `try_wait` observed an exit; a child that survived the kill keeps the
write end open, so reading it would block forever. If the child did not exit,
the parent does not read the pipe and synthesizes the timeout detail. The cap
matters because an unbounded detail can fill the anonymous-pipe buffer (4 KiB
on Windows) and block the child, turning a working probe into a 10 s timeout.
The line is newline-flushed by Rust's line-buffered stdout, so a detail printed
before a crash survives.

Alternatives rejected: a sidecar file (stale-file lifecycle, extra C++ reads);
the child writing `gpu-backend.json` (breaks the "child leaves state untouched"
contract and the parent owns cache writes); the parent re-creating a device to
read adapter info (duplicates device creation and cannot capture failure text).

### D3. Error capture is a process-local slot

`request_device` currently discards errors with `.ok()?`. It records the
`RequestAdapterError` / `RequestDeviceError` text (and the software-adapter and
storage-buffer rejections) into a `Mutex<Option<String>>`; the probe takes it
after a failed device check. `gpu_composite_verified` records a validation
mismatch. The slot is process-local and only read by the probe child, so it
does not affect the parent's cached device path.

The detail is formatted by a pure function so it is testable:
`compiler=<label>; adapter=<name>; driver=<driver>` on success (the label from
D1), and the request error text on failure. The parent stores it with `QString::left(512)`; the cache
file already caps at 64 KiB and a long error cannot grow it unboundedly.

### D4. WARP parity reuses the test feature, renamed

The feature's contract is now "accept any CPU-type device", not "accept
Vulkan". Rename `software-vulkan` to `software-gpu` (Cargo feature,
`SOFTWARE_GPU` const, CI flags, docs, and the `pictura-app` guard test). No
workspace manifest enables it, so production behavior is unchanged.

`windows-render` gains nextest and a step:
`cargo nextest run -p pictura-render --features software-gpu --profile gpu
--run-ignored all`. The `gpu` profile already selects the parity binaries, the
`gpu::`/`gpu_filter::` in-crate checks, and the fixture scenes, and excludes the
benchmarks. The Vulkan-only `vulkan_probe_is_clean_on_a_software_device` test is
gated to non-Windows so `--run-ignored all` does not run it without a Vulkan
driver. The existing default-feature test step stays, so the production
"rejects software" path is still covered.

Alternative rejected: a new `software-dx12` feature. It duplicates the
eligibility switch and `accelerates()` would need a backend parameter for no
additional coverage.

### D5. The cache policy is unchanged

`WGPU_BACKEND` still honors a cached `crashed`. The crash-loop guard stays: a
marker from the parent's own init marks the backend `crashed` and the next
launch skips it. Manual diagnosis uses the existing reset path (re-enable GPU
Compute clears the cache, or delete `gpu-backend.json`). The marker and
parent-init failure paths now write a short detail so the cached state is not
left next to a stale success message.

## Risks / Trade-offs

- **WARP parity is unproven until the first CI run.** The validation scene or a
  float-heavy blend mode may exceed the ±1 LSB tolerance. Mitigation: run the
  job, then either fix the kernel or add a documented skip with a reason; never
  loosen the CPU oracle.
- **FXC compile time.** FXC is slower than DXC, but compilation happens at
  pipeline creation, not per frame. If startup becomes measurable, revisit
  shipping a modern DXC; the policy seam is one function.
- **SM 5.1 ceiling.** A future shader using subgroups or 16-bit types would need
  DXC. The spec records that the default is FXC, so that change is explicit.
- **Windows-only verification.** The WARP run is the only automated proof of
  the FXC path; the reporter's HD 530 remains the real-hardware check.
- **Feature rename churn.** It touches test-only code and docs; no production
  manifest enables the feature, so the risk is limited to a missed reference,
  which the compiler catches.
- **Windows job wall clock.** `windows-render` has a 45-minute timeout and now
  also runs the whole `gpu` profile on WARP. The scenes are small, so the risk
  is low, but the first run decides: raise `timeout-minutes` if it approaches
  the limit rather than dropping coverage.
