# Per-layer GPU eligibility — before/after measurements

## Setup

- Machine: NVIDIA GeForce RTX 3090 (24 GiB), driver 610.57.04, Vulkan 1.4.357.
- Baseline: `e07380d2` (the branch's merge base), built from a clean
  `git archive` with a separate target directory.
- Branch: `perf/1017-per-layer-gpu-eligibility` working tree.
- Profile: `cargo test` test profile (`pictura-render` at opt-level 2), three
  invocations; each invocation takes the minimum of three timed iterations.
- Harness: `crates/pictura-render/tests/per_layer_bench.rs`, an ignored test:

```sh
PICTURA_GPU_BENCH=1 cargo test -p pictura-render --test per_layer_bench -- --ignored --nocapture
```

The fixture is a 4000×4000 RGB/8-bit document with a full-canvas pixel
background, a full-canvas Curves adjustment, a 1024×1024 layer with a drop
shadow, and a 512×512 clipping group (base + clipped layer). On the baseline
any one of the Curves layer, the effect, or the clipping group declines the
whole document to the CPU; on the branch the document composites on the GPU.

## Results

| Path | Baseline `e07380d2` | Branch | Change |
|---|---|---|---|
| Full 4000² composite, active backend | CPU 222.6 ms | GPU 71.3 ms | **3.1× faster** |
| 512×512 region, active backend | CPU 5.52 ms | GPU 1.81 ms | **3.1× faster** |

Medians of three invocations:

| Run | Baseline full | Baseline region | Branch full | Branch region |
|---|---|---|---|---|
| 1 | 220.06 ms | 5.52 ms | 71.48 ms | 2.17 ms |
| 2 | 222.58 ms | 5.49 ms | 71.28 ms | 1.73 ms |
| 3 | 232.72 ms | 5.63 ms | 69.33 ms | 1.81 ms |

Parity against the CPU oracle: full composite 1 LSB max delta, region 0 LSB.
The branch's own GPU-vs-CPU ratio is 3.2× (full) and 2.9× (region); the table
above compares the branch's GPU path to the baseline's CPU fallback, which is
the end-to-end effect of the change.

## Plain GPU path (no new routes)

A 4000², three-pixel-layer document on the pre-existing GPU route, to check
that the per-layer plan did not regress it:

| Baseline `e07380d2` | Branch |
|---|---|
| GPU 61 ms (60, 61, 61) | GPU 58 ms (58, 57, 59) |

The ~5% difference is below run-to-run noise; there is no measurable
regression.

## Notes

- The clipping group is still built on the CPU and only blended on the GPU
  (`design.md`, "Clipping group is CPU-composited"). A clipping-dominated
  document will see less than this fixture, whose 512² group is small next to
  the full-canvas Curves and the 1024² effect layer.
- The effect-pass groups are built on the CPU as well; the GPU win is the
  full-canvas blending and the Curves lookup.
- The branch's CPU oracle is within noise of the baseline (226 ms vs 223 ms),
  so the CPU path is unchanged.
- The 3.1× is below the 3–7× cited for pure composites because the fixture
  includes the CPU-side clipping-group and effect-pass builds.
