# Benchmark: GPU text gamma

Date: 2026-10-28
Branch: `perf/976-gpu-text-gamma` (working tree on `origin/master` @ `08cf561c`)
Host: 16 vCPU, NVIDIA GeForce RTX 3090, driver 610.57.04, Vulkan 1.4.341

## What is measured

A 1024² and a 4000² RGB/8 document: an opaque gradient backdrop plus a
full-canvas proxied text layer whose alpha sweeps 0…255. The type layer carries
a `TypeTool` and a raster proxy, with the gamma at its default 1.45.

Before this change `check_supported` declined such a document, so the CPU path
(`composite_active(doc, false)`) is the old behavior. The new behavior is the
GPU path (`composite_active(doc, true)`). `gpu_off` is the same GPU composite
with the gamma disabled, measuring the curve's own cost.

The source planes are unstamped, so every composite assembles and uploads the
same way a fresh edit would. The device, pipeline, and one warm-up composite
run before the measured runs. Each cell is the median of 5 runs.

## Command

```bash
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=12 \
  cargo test -p pictura-render --features software-vulkan \
  --test gpu_text_gamma_bench -- --ignored --nocapture
```

## Results

Two independent runs (each cell is the median of 5; the spread is run-to-run
variance on a shared desktop host):

| Document | CPU γ on (old path) | GPU γ on (new path) | GPU γ off | Speedup | Curve overhead |
|---|---:|---:|---:|---:|---:|
| 1024² | 16.6–16.7 ms | 4.4–5.0 ms | 4.1–4.4 ms | 3.3–3.7× | 0.3–0.7 ms |
| 4000² | 213–223 ms | 44.0–44.8 ms | 43.3–45.3 ms | 4.8–5.1× | within ±1 ms (noise) |

Raw output:

```text
text-gamma bench 1024x1024 (median of 5, unstamped source): cpu_on=16.6 ms gpu_on=4.4 ms gpu_off=4.1 ms speedup=3.74x curve_overhead=0.3 ms
text-gamma bench 4000x4000 (median of 5, unstamped source): cpu_on=223.3 ms gpu_on=44.0 ms gpu_off=43.3 ms speedup=5.08x curve_overhead=0.7 ms
text-gamma bench 1024x1024 (median of 5, unstamped source): cpu_on=16.7 ms gpu_on=5.0 ms gpu_off=4.4 ms speedup=3.32x curve_overhead=0.7 ms
text-gamma bench 4000x4000 (median of 5, unstamped source): cpu_on=213.2 ms gpu_on=44.8 ms gpu_off=45.3 ms speedup=4.76x curve_overhead=-0.5 ms
```

## Interpretation

- The document stays on the GPU instead of falling back to the CPU: about 5×
  faster at 4000² on this host, and the curve adds no measurable time beyond
  run-to-run noise (<1 ms).
- The CPU path is row-parallel on 16 threads here, so its 213–223 ms is lower
  than the single-run 1064 ms figure quoted in
  `docs/dev/app-bugs-investigation.md:326`. The GPU is also faster than the
  190 ms reference on this RTX 3090. Absolute numbers are host-specific; the
  CPU/GPU ratio is the transferable result.
- The LUT is 16 KB and is uploaded once per composite; its cost is inside the
  `gpu_on` column and is not visible above the noise.

## Correctness evidence

The GPU parity suite, including every new text scene, passes on the same host:

```text
test text::proxied_type_layer_at_default_gamma_matches_cpu ... ok
test text::proxied_type_gamma_values_and_gates_match_cpu ... ok
test text::proxied_type_in_groups_matches_cpu ... ok
test text::proxied_type_blend_modes_match_cpu ... ok
test region::region_matches_full_slice ... ok
test region::full_region_equals_full_composite ... ok
13 passed; 0 failed
```

`cargo test -p pictura-render --lib` also passes: 1226 passed, 0 failed.
