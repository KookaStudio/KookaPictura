# Benchmark — share-history-planes (#973)

The benchmark is the ignored unit test
`crates/pictura-app/src/cxxqt_object/tests_profiles.rs`,
`large_document_structural_edit_profile`. It builds a 6000×4000 RGB/8 document
with 20 full-canvas pixel layers (4 planes × 24 MB = 96 MB each) and an RGBA
composite (96 MB), so the base document is 2.02 GB. It captures Open, then for
20 rounds adds a New Layer, captures it, and undoes/redoes it, printing each
round's add, capture, and undo times and the retained tracked-plane bytes.

## Reproduce

```bash
cargo test --release -p pictura_app large_document_structural_edit_profile -- \
  --ignored --nocapture
```

## Environment

- 11th Gen Intel Core i7-11700K @ 3.60 GHz, 16 hardware threads
- Linux, Rust 1.98.1, release profile (`--release`)
- One run; per-round numbers are stable except the first/last round

## Results (release)

| Phase | Min | Median | Max |
|---|---|---|---|
| `add_layer` | 33.59 ms | 36.40 ms | 45.96 ms |
| structural capture | 16.33 ms | 22.90 ms | 36.43 ms |
| add + capture | 52.84 ms | 59.57 ms | 73.09 ms |
| undo across the anchor | 0.07 ms | 0.12 ms | 0.20 ms |

- `structural_open` (the first capture, still a full `private_copy`): 528.58 ms
  for the 2.02 GB base document, i.e. about 3.82 GB/s on this machine.
- Retained tracked-plane bytes after 20 New Layers: **3.94 GB** (2.02 GB base +
  20 × 96 MB added layers), against the issue's 5 GB target.

## Baseline the branch replaces

The old path ran `private_copy` of the whole document on every structural
capture and every cross-anchor undo. The Open capture above measures that exact
operation on this machine: 528.58 ms for 2.02 GB. The old structural capture of
add `k` copied the post-add document (2.11–3.94 GB), so about 0.55–1.03 s per
add; the old undo copied the target document (2.02–3.84 GB), so about
0.53–1.01 s per undo.

The retention bound is arithmetic: the old history kept 20 full states of
20–39 layers plus the current 40-layer document and 21 RGBA composites, about
**62.5 GB**. The side-by-side result:

| Metric | Old `private_copy` path | This branch | Improvement |
|---|---|---|---|
| Per-add capture | ~0.55–1.03 s | 16–36 ms (median 23 ms) | ~23–45× |
| Undo across the anchor | ~0.53–1.01 s | 0.07–0.20 ms | >5,000× |
| Add + capture | ~0.59–1.08 s | 53–73 ms | ~8–15× |
| Retained after 20 adds | ~62.5 GB | 3.94 GB | ~16× |

## Against the issue targets

- Undo of an add under 30 ms: **passes** (0.2 ms worst).
- Retained memory after 20 New Layers under 5 GB: **passes** (3.94 GB).
- Add layer plus capture under 20 ms: **misses on this machine** (53–73 ms).
  The capture itself is about 23 ms, which is the cost of copying the one new
  96 MB layer at this machine's measured ~4 GB/s; `add_layer` alone costs about
  36 ms because it allocates four fresh 24 MB planes and copies each `Vec` into
  its refcounted allocation. That allocation cost is separate from #973; on the
  issue's 12 GB/s figure the same 96 MB capture would be about 8 ms and the
  add+capture would sit near the target.

The first capture of a document (`structural_open`) still copies it once; that
is the baseline every later capture now avoids. Snapshots, crop/canvas-size
captures, and native-depth `Samples` remain outside this benchmark and outside
the change, as recorded in the design's non-goals.
