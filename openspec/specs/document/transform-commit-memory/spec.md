# document/transform-commit-memory Specification

## Purpose
A Free Transform commit holds no more memory than it needs: a whole-pixel move keeps the layer's planes, the resampler allocates fallibly, and the commit gate charges a move only for its composite.

## Requirements

### Requirement: An integer move keeps the layer's tiles

When a layer similarity transform has scale 1, angle 0 and whole-pixel offsets,
and its resolved interpolation is not Bicubic Smoother, the transform SHALL NOT
resample. The layer's channels SHALL keep their existing planes under the rect
shifted by the offset. A layer mask SHALL shift by the same offset with its data
unchanged. The result SHALL be byte-identical to the resampled result. A Free Transform
session with no linear part SHALL pass its offset through unchanged, whatever its
reference point, so that a whole-pixel move takes this path.

#### Scenario: A move shares the source planes [tcm_move_shares_tiles]

- **WHEN** a tiled layer with a mask is moved by (37, -21) with Bilinear interpolation
- **THEN** every channel shares its planes with the source, the rect and the mask rect are shifted by (37, -21), and the pixels equal the source pixels

#### Scenario: A move matches the resampled result [tcm_move_matches_resample]

- **WHEN** a layer is moved by a whole-pixel offset with each interpolation, once through the fast path and once through the resample path with the same map
- **THEN** for every interpolation except Bicubic Smoother, the two results have the same rect and identical channel and mask pixels

#### Scenario: A session move about any reference point stays exact [tcm_session_move_exact]

- **WHEN** a Free Transform session with scale 1, no rotation or skew, a reference point at (0.1, 29.3) and an offset of (7, −3) is applied
- **THEN** the layer rect moves by (7, −3) and every channel shares its planes with the source

#### Scenario: Bicubic Smoother still resamples [tcm_smoother_resamples]

- **WHEN** a layer is moved by a whole-pixel offset with Bicubic Smoother
- **THEN** its channels do not share planes with the source

### Requirement: Transform resample buffers are allocated fallibly

The layer transform resampler SHALL build its padded source and output buffers
with `memory::try_plane`. When an allocation fails, the transform SHALL refuse,
leave the layer unchanged and record the refusal.

#### Scenario: A failed allocation refuses the transform [tcm_resample_fallible]

- **WHEN** the plane failpoint is armed and a layer is rotated by 10°
- **THEN** the transform returns false, the layer's rect and pixels are unchanged, and a memory refusal is recorded

#### Scenario: A move allocates no resample buffer [tcm_move_no_alloc]

- **WHEN** the plane failpoint is armed and a layer is moved by a whole-pixel offset
- **THEN** the move succeeds and the failpoint is still armed

### Requirement: The commit gate charges a move no layer planes

`commit_bytes` SHALL charge a quad that is the original rect moved by one
whole-pixel offset only for the composite of the union of the old and new boxes,
and SHALL charge any other quad for the layer planes as well.

#### Scenario: A whole-pixel move is charged only for the composite [tcm_gate_move]

- **WHEN** `commit_bytes` estimates a 10 × 10 rect moved by (5, 0), and the same rect moved by (5.5, 0)
- **THEN** the first estimate is `12 · 150` and the second is `4 · 110 + 12 · 160`
