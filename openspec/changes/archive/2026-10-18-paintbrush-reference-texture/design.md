# Design: paintbrush-reference-texture

## Context

The archived `paintbrush-types-pen-tip`, `paintbrush-preset-defaults`, and
`pencil-grain-texture` changes added the four presets and the fixed paper
grain. The owner's reference image shows the `2B Pencil` and `Non-Photo Blue
Pencil` strokes at a larger zoom than the canvas: the 2B core averages about
75 % alpha and the blue core about 62 %, both with 2–3 px clumps and a soft
edge. The preset sizes (10 px and 9 px) come from Photoshop's options bar and
stay unchanged; the reference's larger absolute width is a zoom artefact.

## Decisions

### Multiscale noise

`fill::grain_noise(x, y)` mixes the existing `pixel_noise` hash with bilinear
value noise on 2 px and 3 px grids, then expands the contrast by 1.5:

    noise = clamp((0.3 × fine + 0.3 × smooth2 + 0.4 × smooth3 − 0.5) × 1.5 + 0.5, 0, 1)

The fine term keeps the texture from reading as a smooth blotch; the two
value-noise terms form the reference's 2–3 px clumps. The result is
deterministic, in `0.0..=1.0`, and keyed to layer coordinates, so overlapping
dabs see the same pattern and a stroke always renders identically. No
threshold holes are introduced: the reference's light patches come from the
noise reaching low values, not from dropped pixels.

### Strength and edge

`keep = 1 − grain% × (1 − noise)`. The mixed noise has mean 0.48 and standard
deviation 0.21, so a 50 % grain leaves about 74 % mean core coverage and a
75 % grain about 61 %. Those match the reference's 2B and blue interiors.
Hardness 65 % gives the reference's soft edge transition while keeping a solid
core; the tips stay anti-aliased, and the Pencil tool's own aliased mode is
unchanged. Both pencils use the same noise field; the preset grain strength is
the only difference.

### Tests

- `fill::grain_noise_is_deterministic_and_clumpy`: values stay in range, repeat
  exactly, and a 2 px box average keeps most of the texture's spread.
- `stroke::grain_breaks_up_the_dab_alpha`: the grained dab has partial alpha,
  no zero-alpha core pixels, less total coverage, and repeats exactly.
- `stroke::grain_strength_matches_the_pencil_presets`: grain 50 leaves roughly
  74 % and grain 75 roughly 61 % of the plain dab's coverage.
- `tst_brush_tip_preview`: the two presets report hardness 65 and grain 50 /
  75.
