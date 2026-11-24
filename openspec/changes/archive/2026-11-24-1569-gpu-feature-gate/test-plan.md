# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| bp_gpu_default_off | Read `crates/pictura-render/Cargo.toml`, then run `cargo nextest run -p pictura-render` with no features | `wgpu`/`pollster` are optional, `default` is empty, and the crate compiles and tests without either dependency in the graph | `pictura-render::renderer_manifest_gates_wgpu_behind_the_gpu_feature` |
| bp_gpu_surface | Compile `pictura-render` with `--features gpu` and assign `pictura_render::gpu::Backend::Cpu` to a `pictura_render::Backend` binding, then read every root re-export the app uses | The two names are one type and each documented symbol resolves | `pictura-render::gpu_backend_is_one_type_behind_the_feature` |
| bp_gpu_cpu_composite | Composite a small document through `CompositeRequest::whole(doc, true)` in a build without `gpu` | Bytes equal `composite_rgba` and the reported backend is `Backend::Cpu` | `pictura-render::gpu_enabled_request_falls_back_to_the_cpu_without_the_feature` |
| bp_gpu_app_enables | Read `crates/pictura-app/Cargo.toml` | The `pictura-render` dependency carries `features = ["gpu"]`; the dev-dependency stays `test-oracle`-only | `pictura-render::app_manifest_enables_the_gpu_feature` |
| bp_gpu_software | Read the renderer manifest's `software-gpu` definition and the `gpu` nextest profile command | `software-gpu` contains `gpu`; the documented command enables both | `pictura-render::software_gpu_implies_gpu` |
| bp_gpu_tests | Inspect `require_gpu`/`require_device` and run the `gpu` profile on a host with no adapter | The device-dependent test asserts and fails; it never returns early as a pass | manual: a failing run needs a host with a `gpu` build and no Vulkan device, which this worktree cannot provide; the check is the existing `require_gpu` assertion in `crates/pictura-render/src/gpu/mod.rs` |

The two feature-state tests are compile-time gated so each build runs the one it
can prove: `gpu_backend_is_one_type_behind_the_feature` is
`#[cfg(feature = "gpu")]` and
`gpu_enabled_request_falls_back_to_the_cpu_without_the_feature` is
`#[cfg(not(feature = "gpu"))]`. The default-feature
`cargo test -p pictura-render --lib feature_gate` therefore runs the manifest,
app-manifest, software-gpu and CPU-fallback tests; the `--features gpu` run adds
the surface test.
