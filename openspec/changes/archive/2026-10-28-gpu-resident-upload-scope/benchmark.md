# Benchmark — resident upload scope

Run with:

```bash
cargo test -p pictura-render --test resident_upload_profile -- --ignored --nocapture
```

The test is `#[ignore]`d and is not part of the gpu-parity CI profile. It skips
with a printed note when no Vulkan device is usable.

## Setup

- Debug build, Vulkan on an NVIDIA RTX 3090.
- 4000×4000 RGB document, 8 full-canvas pixel layers, no masks.
- 300 region composites over moving 512×512 rects.
- One layer is written every present (its stamp clears), modelling the active
  layer of a live stroke; the other seven keep their stamps.
- The written layer is re-stamped after the stroke, then the whole document is
  composited again.
- The pre-change numbers come from the same test run against `HEAD`
  (`08cf561c`, the branch point), with the gauge prints removed because that
  tree has no `resident_entries`/`resident_bytes` counters.

Three runs each; the medians and the observed range are below.

## Results

| Measurement | Before (`HEAD`) | After | Change |
|---|---|---|---|
| Region present | 11.26 ms median (10.86–13.59) | 7.45 ms median (6.24–8.17) | ~1.5× faster |
| Full composite after the stroke | 154.68 ms median (149.88–163.47) | 90.49 ms median (84.21–113.73) | ~1.7× faster |
| Resident store after 300 presents | fills toward the 2 GB budget (2,400 puts) | 8 entries / 512,000,000 B, flat | bounded |
| Resident source uploads over 300 presents | 0 source hits / 2,100 source misses | 2,100 source hits / 0 source misses | reuse |
| Shared constant-coverage mask over 300 presents | 2,100 mask hits / 300 mask misses | rebuilt, no counters | memory/upload trade-off |

The profile prints the combined `resident_hits`/`resident_misses`; the split
above follows the per-present dispatch: the seven unchanged sources hit or miss,
the written layer's source is unstamped (neither hit nor miss), and one shared
mask key serves all eight maskless layers in the before path. The 2,400 puts
are 2,100 source misses plus 300 mask misses.

The final full composite after the stroke:

| | Before | After |
|---|---|---|
| Source uploads | 8 sources (512 MB) | 7 hits + 1 source (64 MB) |
| Mask uploads | 1 shared mask (16 MB) | 8 constant masks rebuilt (128 MB) |
| Counter deltas | +7 hits, +9 misses | +7 hits, +1 miss |

The before store after the stroke is at the 2 GB budget because each present
keeps 7 region-keyed sources (7 MB) plus one shared region mask (0.25 MB): about
7.25 MB per present, or ~2.2 GB over 300 presents. The after store stays at the
512 MB initial whole-document upload for all 300 presents and grows to 576 MB
after the final composite adds the re-stamped active layer's new source.

## Read this honestly

- Present time is dominated by dispatch and readback, not by the upload it
  saves. The structural win is the flat store and the hit rate; the timing
  improvement is real but machine- and build-dependent (debug build here, one
  GPU, three runs).
- The before full-composite number includes re-uploading every source because
  the stroke's region-keyed entries filled the budget and evicted the
  whole-document uploads. With a shorter stroke that does not reach the budget,
  the before code would still hit the initial whole-document entries.
- The after full composite rebuilds all eight constant-coverage masks; the old
  code kept one shared 16 MB entry for the eight full-canvas layers, while the
  new code uploads 8 × 16 MB per full composite. That is the deliberate
  memory-for-upload trade-off in the design.
