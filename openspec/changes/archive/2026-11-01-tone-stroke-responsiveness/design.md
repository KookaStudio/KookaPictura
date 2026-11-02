# Design

## Where the time went

`PICTURA_METRICS` put nearly all of the lag in `cpu_stroke_sample`, so no stack
sampling was needed. The press paid for a serial 1 GiB write of the coverage record
and a whole-plane copy in `write_pixel`, because every byte access went through
`Plane::as_mut_slice`, which flattens a banded plane. Each later dab rebuilt the tip
per pixel on one thread, then rescanned the union of the sample's dabs.

## Exactness

The serial loop stays as the test-only `dab_serial`. A pixel's result depends only
on that pixel, the tip, and its own coverage record, so rows can run in any order.
The chord path holds across every tip tested, including rotated aliased tips.

## Lazy pixels are Tone-only

Tone reads only inside `dab_bounds`, so `LazyPixels` fills the tiles a dab meets just
before the dab runs. The layer still holds pre-stroke pixels there, because writes
only reach tiles already loaded. Smudge, Blur, Sharpen, and Mixer read a halo around
the dab and keep `layer_rgba`; #1070 tracks them.

## Write-back per dab

A fast drag places dabs far apart, and their union is mostly untouched pixels. Each
dab's rectangle is saved and written on its own. Overlaps compare equal and write
nothing, and the reported bounding box is unchanged.

## Measurements

`scripts/profile-tools.py --moves 30` on `worldmap_16000x16000.png` (16507×16196),
16 cores, both binaries back to back, in milliseconds:

| Scenario | down | move p50 | move p95 | up | undo |
|---|---|---|---|---|---|
| dodge-40 | 1691 → 6.5 | 15.7 → 18.3 | 18.1 → 23.4 | 481 → 356 | 281 → 436 |
| dodge-500 | 549 → 10 | 51.7 → 20.7 | 80.3 → 25.6 | 514 → 394 | 345 → 519 |
| dodge-1940 | 712 → 61 | 139.6 → 38.7 | 264 → 64.3 | 765 → 593 | 794 → 774 |
| sponge-1940 | 702 → 49 | 126.4 → 32.0 | 231 → 58.5 | 712 → 575 | 774 → 778 |
| brush-500, for reference | 48 | 26.7 | 49.2 | 541 | 531 |
| brush-1940, for reference | 79 | 46.5 | 79.9 | 653 | 848 |

Resident size after the last scenario falls from 5.8 GB to 5.3 GB.

## Costs that remain

- Small-stroke undo rose by about 50 %, to the Brush's level: toned planes now stay
  banded, as the Brush's do, so undo takes the same path.
- `dodge-40` moves rose about 2.6 ms.
- Release still costs 350–600 ms in the history commit, as it does for the Brush.
