# Design: port-photorust-fibers

## Model

Ported from perfecto25/photorust `core/src/filters/render.rs::fibers`, with
its idioms kept so the port diffs against upstream.

- **Octaves.** `BANDS = [48, 20, 8, 2, 1]` px across. The first three are
  *clumps* (weights falling by 0.6), the last two *hairs* (weights rising by
  1.7). Hair lattices land on every pixel or every other one, so hairs come
  out hard-edged while clumps stay smooth. Hard edges at every octave give a
  mosaic of blocks.
- **Run length.** Each octave samples `noise2(x / band, (y + phase) /
  (run · √band · length))`. The clump run is `60 / (1 + 0.02·variance)` and
  the hair run is `28 / (1 + 0.04·variance)`, both scaled by `√(strength / 4)`.
  `phase` and `length` (0.6–1.6) are drawn per column and octave from that
  octave's lattice. A shared phase gives corduroy.
- **Mix.** `hair_share = 0.20 + 0.45·(variance / 64)^0.6`.
- **Tone.** The field is stretched about its mean by
  `(0.55 + 0.95·√(variance / 64)) / (4.4·σ)`, clipped to 0..1, and blended
  from `color_b` (0) to `color_a` (1). Alpha is untouched.

## Kooka adaptations

- The planar `PixelBuffer` is written directly, plane by plane, rather than
  through the interleaved `Pixmap`. Nothing else needs it.
- Seeds fold `u64 → u32` as `lo ^ hi`, so seeds below 2³² are photorust's
  seeds and Randomize still re-rolls.
- `noise01` and `noise2` are private copies in `render/fibers.rs`. Kooka's
  photorust module has `hash2`, which is the same hash without the `% 65521`
  scale, but Fibers is the only user of the 2D value noise.
- Out-of-range parameters are rejected (`FilterError::InvalidParams`).
  photorust clamps them, but the existing Kooka contract rejects them.

## Clouds

Out of scope by decision. Kooka's Clouds already re-rolls per seed, takes two
colours, and has Starker. photorust's `Clouds { difference }` does neither.
`DifferenceClouds` stays a separate variant, so the Filter menu, the
`filter_map.rs` kinds, and the self-test are untouched.
