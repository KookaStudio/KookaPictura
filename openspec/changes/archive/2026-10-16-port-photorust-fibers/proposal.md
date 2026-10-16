# Proposal: port-photorust-fibers

## Why

Kooka's Fibers runs streaks across the picture, not down it. It is one fBm
field squeezed along x with a per-row jitter, so it reads as grey horizontal
cloud instead of CS6's vertical wands and hairs. Its ranges (Variance 0..=100,
Strength 1..=100) also disagree with its own dialog, which offers CS6's 1–64.
photorust's Fibers (#183) was tuned by eye against CS6 at 500 %. It lays crisp
one- and two-pixel hairs over soft clumps tens of pixels wide, all running
down the picture. Issue #224 asks to port it.

## What Changes

- `render::fibers` becomes photorust's streak model, moved to
  `render/fibers.rs`. Three smooth clump octaves (48, 20, 8 px) and two
  hard-edged hair octaves (2, 1 px) are stretched vertically. Each column
  gets its own phase and run length, and the field is contrast-stretched
  about its mean to fill the colour range.
- `color_a` maps to photorust's foreground and `color_b` to its background.
  The `u64` seed is folded to photorust's `u32`, so seed 0..999 is unchanged.
- Variance accepts `0..=64` and Strength `1..=64`, CS6's slider ranges.
  Variance 0 is the even blend of the two colours. Values outside the range
  are rejected as before.
- Clouds and Difference Clouds stay as they are. They keep Kooka's seed,
  colours, Starker, and separate `DifferenceClouds` variant. photorust's
  `Clouds { difference }` has no seed, so it cannot re-roll as CS6 does.

## Capabilities

### Modified Capabilities

- `imaging/render-filters`: Fibers runs vertically as clumps and hairs, and
  its ranges narrow to CS6's 0..=64 and 1..=64.

## Impact

- `pictura-filters` `render` only. The `Filter::Fibers` fields, the
  `fibers` kind, its defaults (variance 16, strength 4, seed 1), and the
  dialog are unchanged.
- **Output changes:** every Fibers result changes. No golden baseline covers
  Fibers.
- **Oracle:** Fibers stays no-equivalent and is covered by property tests
  ported from photorust.
- **Docs:** `docs/06-filters/render-filters.md` describes the streak model and
  the 1–64 ranges, in a separate `TASK-ALLOWS-DOCS` commit.
