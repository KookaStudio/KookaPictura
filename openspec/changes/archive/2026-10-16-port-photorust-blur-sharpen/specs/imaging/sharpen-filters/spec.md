## MODIFIED Requirements

### Requirement: Sharpen and Sharpen More

The system SHALL implement `Filter::Sharpen` and `Filter::SharpenMore` with no parameters, as Unsharp Mask at fixed settings: radius `sharpen::SHARPEN_RADIUS` (3.0, σ 1), threshold 0, and amount `sharpen::SHARPEN_AMOUNT` (50 %) for `Sharpen` and `sharpen::SHARPEN_MORE_AMOUNT` (100 %) for `SharpenMore`. A uniform-color buffer SHALL be unchanged. `SharpenMore`'s deviation from the input SHALL be at least as large as `Sharpen`'s on the same edge image. Repeated application SHALL add contrast gradually rather than amplifying fine detail without bound. Oracle expectation: differential against ImageMagick `-unsharp 0x1+0.5+0` for `Sharpen` within tolerance 2 and `-unsharp 0x1+1+0` for `SharpenMore` within tolerance 3.

#### Scenario: The kernel response is a known value

- **WHEN** `Sharpen` and `UnsharpMask { amount: SHARPEN_AMOUNT, radius: SHARPEN_RADIUS, threshold: 0 }` are applied to the same edge buffer, and likewise `SharpenMore` with `SHARPEN_MORE_AMOUNT`
- **THEN** each pair of outputs is byte-identical, and a flat neighborhood is unchanged

#### Scenario: Sharpen More is stronger

- **WHEN** `Sharpen` and `SharpenMore` are each applied to the same edge image
- **THEN** the total deviation from the input is larger for `SharpenMore` than for `Sharpen`

#### Scenario: Repeated Sharpen stays bounded

- **WHEN** `Sharpen` is applied three times to a fine low-contrast pattern
- **THEN** the contrast grows but stays well short of the full tonal range

#### Scenario: Matches ImageMagick

- **WHEN** `Sharpen` is applied to the oracle image and ImageMagick applies `-unsharp 0x1+0.5+0`, and `SharpenMore` against `-unsharp 0x1+1+0`
- **THEN** no sample differs by more than 2 for `Sharpen` and 3 for `SharpenMore`

### Requirement: Sharpen Edges

The system SHALL implement `Filter::SharpenEdges` with no parameters. It SHALL measure edge strength as the Sobel gradient magnitude of the Rec.601 brightness of a σ 1 Gaussian blur of the image, and SHALL add `original − blurred` per colour channel, weighted by a smoothstep of that strength between fixed floor and ceiling levels (6 and 36). Below the floor the pixel SHALL be unchanged, so flat regions keep their values. Oracle expectation: no faithful ImageMagick equivalent exists (an edge-gated sharpen), so property tests cover the flat-region no-op and the edge contrast increase, and the divergence is documented.

#### Scenario: Flat regions are unchanged

- **WHEN** `SharpenEdges` is applied to a buffer with a flat area wider than the blur and Sobel reach
- **THEN** the flat-area samples beyond that reach are bit-exactly unchanged

#### Scenario: Edge contrast increases

- **WHEN** `SharpenEdges` is applied to a step edge
- **THEN** the contrast across the edge increases relative to the input

### Requirement: Sharpen ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py` and `crates/pictura-filters/tests/oracle.rs`. The mapping table SHALL have exactly one row per `Filter` variant. Unsharp Mask, Sharpen, and Sharpen More SHALL be diffed against `-unsharp` within their stated tolerances. SharpenEdges SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0 with known-value or property tests, including the USM-not-an-edge-detector behavior. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every sharpen variant

- **WHEN** the oracle tests run
- **THEN** every sharpen variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes
