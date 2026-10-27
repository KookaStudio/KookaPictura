# Spec Delta

## Purpose

One shared engine for the mask algorithms that selections, filters, paint tools
and layer effects reuse, with exact results and cost independent of the radius
or the number of reference colours.

## ADDED Requirements

### Requirement: Shared square-footprint morphology

The engine SHALL provide one separable dilate/erode over a square structuring
element of side `2 * radius + 1`, usable on an 8-bit coverage plane and on a
32-bit coverage matte. The border policy MUST be an explicit parameter: `zero`
samples outside the plane read zero; `clamp` samples outside read the nearest
edge sample. Radius 0 MUST be the identity. For a given radius and border policy
the output MUST be identical, sample for sample, to the naive square scan over
the same window, including planes narrower or shorter than the radius. The cost
per output sample MUST NOT grow with the radius.

#### Scenario: Zero radius is the identity [msk_morph_radius_zero]

- **WHEN** dilate or erode runs with radius 0
- **THEN** the plane is returned unchanged

#### Scenario: The zero border erodes inward from the canvas edge [msk_morph_zero_edge]

- **WHEN** the all-selected coverage plane is eroded with the zero border
- **THEN** every side shrinks inward by the radius instead of staying selected along the canvas border

#### Scenario: The clamp border keeps the canvas edge [msk_morph_clamp_edge]

- **WHEN** the all-selected coverage plane is eroded with the clamp border
- **THEN** every border sample stays at full coverage

#### Scenario: Random planes match the naive square scan [msk_morph_random]

- **WHEN** dilate and erode run at several radii and both border policies on random planes, including planes smaller than the radius
- **THEN** every sample equals the naive per-pixel square scan's result

#### Scenario: Morphology cost is independent of the radius [msk_morph_scale]

- **WHEN** the same plane is dilated at radius 1 and at radius 100
- **THEN** the per-sample cost stays within a constant factor instead of growing with the radius

### Requirement: Shared coverage median

The engine SHALL provide a square-window median over an 8-bit coverage plane
with clamp-to-edge sampling: each output sample MUST be the median of the
`(2 * radius + 1)²` clamped samples. Radius 0 MUST be the identity. The output
MUST equal sorting each clamped window, byte for byte. The cost per output
sample MUST NOT grow with the radius.

#### Scenario: The median matches sorting each clamped window [msk_median_sorted]

- **WHEN** the shared median runs at several radii on random planes, including at the top, bottom, left and right edges
- **THEN** every sample equals the middle element of its sorted clamped window

#### Scenario: Zero radius changes nothing [msk_median_radius_zero]

- **WHEN** the shared median runs with radius 0
- **THEN** the plane is returned unchanged

#### Scenario: The binary majority rule still holds [msk_median_majority]

- **WHEN** a single selected pixel sits in an otherwise unselected neighbourhood, and separately a single unselected pixel sits inside an otherwise selected neighbourhood, with radius 1
- **THEN** the isolated selected pixel becomes unselected and the lone hole becomes selected

#### Scenario: Median cost is independent of the radius [msk_median_scale]

- **WHEN** the same plane is median-filtered at radius 10 and at radius 100
- **THEN** the per-sample cost stays within a constant factor instead of growing with the radius

### Requirement: Scanline four-connected flood

The engine SHALL provide a four-connected flood that returns a 0 / 255 mask of
the pixels reachable from the initial selected set (one or more seed pixels) and
accepted by a caller-supplied predicate. The returned mask MUST include the
seeds. A caller that starts from soft coverage MUST preserve that coverage on
pixels it already had selected. Each accepted pixel MUST be visited once and
MUST never be revisited. The mask MUST equal a naive breadth-first flood over
the same seeds and predicate. A seed set with no in-plane source MUST return no
mask rather than panic. The frontier storage MUST be bounded by the number of
contiguous runs on the frontier, not by the number of visited pixels.

#### Scenario: The flood matches a naive breadth-first fill [msk_flood_naive]

- **WHEN** the flood runs on random planes against a reference predicate
- **THEN** the returned mask equals the naive four-connected breadth-first result

#### Scenario: The flood stops at a predicate boundary [msk_flood_boundary]

- **WHEN** the seed lies in a region whose neighbours fail the predicate
- **THEN** only the connected accepted region is marked

#### Scenario: A full plane does not build a per-pixel frontier [msk_flood_frontier]

- **WHEN** a uniform plane is flooded
- **THEN** the mask is fully selected and the peak frontier is bounded by a small multiple of the plane height, not by the number of selected pixels

#### Scenario: An out-of-plane seed returns no mask [msk_flood_bad_seed]

- **WHEN** the seed set has no in-plane source
- **THEN** the flood returns no mask instead of panicking

### Requirement: Similar matches any selected colour within tolerance

`Select > Similar` SHALL add to the selection every pixel whose per-channel
maximum RGB difference (Chebyshev distance) from the colour of any pixel already
selected is less than or equal to the Magic Wand tolerance. Pixels already
covered MUST keep their existing coverage. Selection and image dimensions MUST
match, and a mismatch MUST return an error. The result MUST equal comparing each
unselected pixel against every selected pixel's colour. The cost per pixel MUST
NOT grow with the number of distinct selected colours.

#### Scenario: Many reference colours select the same pixels [msk_similar_many_colours]

- **WHEN** Similar runs on a photographic image whose selection contains thousands of distinct colours
- **THEN** it selects exactly the pixels selected by the per-colour reference scan

#### Scenario: Tolerance 0 selects exact colours only [msk_similar_zero_tolerance]

- **WHEN** the tolerance is 0 and the selection contains a colour
- **THEN** only pixels with exactly that colour are added

#### Scenario: Soft selected coverage is preserved [msk_similar_soft]

- **WHEN** a selected pixel has intermediate coverage and a neighbouring pixel is within tolerance
- **THEN** the selected pixel keeps its intermediate coverage and the neighbour becomes fully selected

#### Scenario: A dimension mismatch is an error [msk_similar_size_mismatch]

- **WHEN** Similar runs with a selection and image of different dimensions
- **THEN** it returns an error instead of panicking or truncating

#### Scenario: An empty selection adds nothing [msk_similar_empty_selection]

- **WHEN** Similar runs on an empty selection
- **THEN** no pixel is added and the selection is unchanged

#### Scenario: Similar cost is independent of the colour count [msk_similar_scale]

- **WHEN** Similar runs with ten reference colours and with ten thousand
- **THEN** the per-pixel cost stays within a constant factor

### Requirement: Every adopted call site keeps its documented results

Each operation that adopts the shared engine SHALL keep the results required by
its own capability, with the border policy its behaviour requires: Expand,
Contract and Border use the zero border; Smooth and Refine Edge's Smooth use the
clamp border; Maximum, Minimum, the Median filter, the paint selection stroke and
the layer-effect spread and choke use the clamp border. The existing selection
and filter oracles MUST stay byte-identical. The layer-effect spread and choke
outputs MUST equal the retained pre-migration scan.

#### Scenario: The selection modify operations are unchanged [msk_site_selection]

- **WHEN** Expand, Contract, Border and Smooth run on the existing fixtures
- **THEN** they produce the same selections as before, including the canvas-edge contract and the centred border band

#### Scenario: Filter digests are unchanged [msk_site_filters]

- **WHEN** the Maximum, Minimum and Median filters run on the fixed filter picture
- **THEN** their output digests equal the recorded digests

#### Scenario: Layer-effect outputs are unchanged [msk_site_effects]

- **WHEN** the retained pre-migration spread and choke scan is compared after the matte migration
- **THEN** every sampled output equals the retained pre-migration scan

#### Scenario: Magic Wand and Grow are unchanged [msk_site_wand_grow]

- **WHEN** Magic Wand runs in contiguous and global modes and Grow expands a selection
- **THEN** their selections match the previous results, and Grow preserves existing intermediate coverage

#### Scenario: The paint flood and stroke are unchanged [msk_site_paint]

- **WHEN** the Paint Bucket floods a surface and the selection stroke builds its band
- **THEN** the masks equal the previous results

### Requirement: Parallel execution is deterministic

Operations that parallelise over rows or strips SHALL produce the same output
for any thread count and any scheduling. No result may depend on the order in
which rows finish.

#### Scenario: One thread and many threads agree [msk_parallel_deterministic]

- **WHEN** an operation runs in a single-threaded pool and in a multi-threaded pool on the same input
- **THEN** the two outputs are identical

### Requirement: Full-document latency targets

On a 6000×4000 single-channel mask, the profiled operations SHALL meet these
targets: Similar with 10 000 reference colours under 100 ms; Smooth at radius
100 under 1 s; Expand at radius 100 under 300 ms. The profile harness MUST
report each timing so a regression is visible.

#### Scenario: Similar meets its target [msk_perf_similar]

- **WHEN** the full-document profile runs Similar with 10 000 reference colours
- **THEN** it reports a time under 100 ms

#### Scenario: Smooth meets its target [msk_perf_smooth]

- **WHEN** the full-document profile runs Smooth at radius 100
- **THEN** it reports a time under 1 s

#### Scenario: Expand meets its target [msk_perf_expand]

- **WHEN** the full-document profile runs Expand at radius 100
- **THEN** it reports a time under 300 ms
