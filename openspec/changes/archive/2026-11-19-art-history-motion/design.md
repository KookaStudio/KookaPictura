# Design

## Measurement

Host: AMD Ryzen 7 255 (8 cores / 16 threads), Radeon 780M, 29 GiB, Fedora 43
(Linux 6.19), Qt 6 system build, CMake RelWithDebInfo with the `dev-app`
profile. Other worktrees were building, so load averaged 3–10; each figure is
a range over three runs. Command:

```sh
python3 scripts/profile-tools.py --size 8000x6000 --gpu on --moves 60 --memory-max 8G --metrics \
  --scenarios art-history,art-history-textured,history-brush,history-brush-textured
```

Motion p95 (ms):

| Scenario | Baseline (master) | This change |
|---|---:|---:|
| `art-history` (uniform, nothing paints) | 52.7 | 7.6–8.1 |
| `art-history-textured` | 75.1 | 8.7–10.2 |
| `history-brush-textured` (reference) | 9.6 | 7.5–8.1 |

Gesture totals went from 1913 to 326–402 ms (uniform) and from 3010 to
506–519 ms (textured).

## Attribution

- **Dab math.** With the uniform source no pixel changes and nothing is
  presented, yet `cpu_stroke_sample` was 1225 ms per gesture, about 10× History
  Brush. An engine-only replay of the harness stroke (`motion_profile`, ignored)
  reproduced it at about 17 ms per move. Each stamp evaluated
  `tip_coverage(cfg, …)` per pixel, rebuilding `TipParams` (including a
  `sin_cos`) every time. A 200 px tip with Tight Short lays 7 stamps per dab,
  and a move lays about 3 dabs, so a move evaluated about 840k pixels
  serially.
- **Undo save.** `Saved::save` read each 64 px tile with 64 one-row plane
  reads. At this size that was 91 ms per gesture; one read per tile brings it
  to 51 ms. The rest is the first touch of the zero-filled save planes.
- **Present.** Region refreshes composited on the GPU. On this machine a region
  composite costs about 1.1 ms per layer on the GPU at any size; the CPU was
  4–10× faster at every side up to 4096 px. `TilePyramid::repair` built level
  0's straight rows even when zoomed out, with no level-0 tile cached, copied
  each level before halving it, and interleaved serially.

`ArtHistoryBrush::dab` was about 2% of a whole-run CPU sample in the issue,
because that sample included document creation, press and undo. Timed over
motion alone, the dab dominated.

## Decisions

- **One pass per stroke, not per tip.** A stroke's tip positions depend only on
  its random walk, not on the pixels, so its tips can be stamped together: each
  row applies the tips in order. Each pixel sees the same blends in the same
  order with the same float expressions, so the output is byte-identical to
  tip-by-tip stamping (`one_pass_strokes_match_tip_by_tip_stamping`). The
  stroke-start gate still reads the image after earlier strokes, so strokes
  stay sequential.
- **Rejected: merging overlapping dab boxes before write-back.** It changed
  nothing measurable, and it is unsafe with lazily loaded pixels: a merged box
  covers pixels never loaded.
- **The GPU floor sits at the call site.** `refresh_display_rect` asks for the
  GPU only when the aligned region has at least `REPAIR_PIXELS` (4 MP). The
  render crate's `composite()` keeps its policy, so the GPU parity tests still
  exercise small regions. The floor is fixed, not calibrated per machine,
  because the measured cost is fixed per call and per layer.
- **The hold timer is unchanged.** Art History always paints while held, and
  the Brush, retouch and stamp tools share the same free-running 50 ms timer.
  Holding off while the pointer moves would change all of them.

## Budget

The motion budget compares Art History with History Brush on the same
document, tip, source and drifted layer, on the same machine and build: motion
p95 within 1.25× of History Brush's. The fixes to save, repair and present
also sped up History Brush, which moved the reference. On medians this change
measures 9.94 / 7.86 ms = 1.26×, at the budget within this machine's noise.
The rest scales with painted area: an Art History stroke runs 1.5 tip
diameters from its dab, so a move dirties about twice the pixels.

## Residual ceilings

- **Save first touch.** Saving a tile still faults the layer-sized,
  zero-filled save planes (about 0.85 ms per move here). Compact tile storage
  would remove it, but every Brush pixel reads the saved base by layer index.
- **Area.** Write-back and present cost scales with each move's dirty box.
- **Press and release.** Down stays about 50–100 ms (source surface and layer
  copy, #1094 and #1070). Release stays about 30–60 ms.
