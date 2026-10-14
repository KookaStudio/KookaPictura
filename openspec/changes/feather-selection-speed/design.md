# Design

## Measurements

Release. The 8000x6000 rows are `feather_selection_profile`
(`tests_profiles.rs`, `--ignored --nocapture`) on a rect 5000x3000 at
(1000, 1000); the 3000x3000 rows are the same measurement on a rect 2000x2000
at (500, 500). The before numbers are the direct f64 convolution the crate
shipped, timed on the same masks:

| document   | feather | before      | after   |
|------------|---------|-------------|---------|
| 3000x3000  | 10      | 0.87 s      | ~0.09 s |
| 3000x3000  | 170     | 13.4 s      | ~0.04 s |
| 8000x6000  | 10      | 2.0 s       | ~0.20 s |
| 8000x6000  | 170     | 33.4 s      | ~0.20 s |

The 8000x6000 profile reports 196-204 ms for every radius >= 4 (the profile
runs in ~0.6 s total); radius 0 moves the shape through untouched.

## Box stack

A Gaussian of sigma has variance `sigma^2`; an odd box of width `w` has
variance `(w^2 - 1) / 12`. Three boxes per axis, the first `m` narrow (`wl`)
and the rest `wu = wl + 2`, with

    wl = largest odd <= sqrt(12 sigma^2 / 3 + 1)
    m  = round((12 sigma^2 - 3 wl^2 - 12 wl - 9) / (-4 wl - 4))

give a total variance closest to `sigma^2`, so the ramp's width and 50 %
crossing match the direct Gaussian. A sliding running sum makes each pass
`O(1)` per pixel; the vertical pass keeps one sum per column so rows are read
sequentially. Rounded 8-bit buffers ping-pong between passes: a 267 MP
document needs 534 MB instead of the direct path's 2.1 GB f64 scratch.

The approximation deviates from the direct truncated Gaussian most at the
cutover: on a hard step every tested radius (4, 8, 20, 100, 250) stays within
4 levels, and a disk (every edge orientation, the worst case) within 6
(`box_stack_tracks_the_gaussian_on_a_hard_edge`). The deviation can grow for
high-frequency masks and at a clamped canvas edge; the kernel is inferred, so
the ramp shape, not a parity claim, is the contract.

Ceiling (`ponytail:` in `lib.rs`): three boxes; a recursive Gaussian would
track the tails closer.
