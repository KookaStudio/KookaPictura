## ADDED Requirements

### Requirement: Smart blur

The system SHALL implement `Filter::SmartBlur { radius: f64, threshold: f64, quality: Quality, mode: SmartBlurMode }` on the colour planes; alpha SHALL NOT be modified. In `Normal` mode each output pixel SHALL be the Gaussian-weighted mean (σ = radius / 3) of the taps within the disc of `radius` whose every colour channel differs from the centre by at most `threshold` levels, with clamp-to-edge borders. `quality` Draft, Good and Best (CS6's Low, Medium and High) SHALL sample the disc with a step chosen so each side carries at most 3, 6 or 12 taps respectively. A pixel SHALL be an edge when any 4-neighbour differs from it by more than `threshold` levels in some channel. `EdgeOnly` SHALL write 255 to every colour plane on edges and 0 elsewhere; `OverlayEdge` SHALL write 255 on edges and the `Normal` result elsewhere. A `radius` or `threshold` outside `0.1..=100` SHALL be refused with `FilterError::InvalidParams`. Oracle expectation: none (ImageMagick's `-selective-blur` gates on luma and draws no edges); unit tests and kernel digests guard it.

#### Scenario: Noise below the threshold smooths and the step survives

- **WHEN** Smart blur in Normal mode is applied to a two-tone step whose halves carry jitter smaller than the threshold
- **THEN** each half's spread shrinks and no sample crosses to the other half's range

#### Scenario: The threshold gates a step

- **WHEN** Normal mode is applied to a 50-level step with thresholds 49.9 and 50
- **THEN** the step is unchanged at 49.9 and blurred at 50

#### Scenario: Edge Only marks the step

- **WHEN** Edge Only is applied to a noisy step whose jitter is below the threshold
- **THEN** the two columns either side of the step are 255 and every other sample is 0

#### Scenario: Overlay Edge is the blur with white edges

- **WHEN** Normal, Edge Only and Overlay Edge are applied with the same parameters
- **THEN** Overlay Edge equals 255 wherever Edge Only is 255 and the Normal result elsewhere

#### Scenario: Quality only matters past its tap budget

- **WHEN** Smart blur is applied at radius 3 with Draft and with Best
- **THEN** the outputs are identical, while at radius 40 Draft samples fewer taps than Good and Good fewer than Best

#### Scenario: Bad parameters are refused

- **WHEN** Smart blur is applied with a radius or threshold of 0, above 100, or NaN
- **THEN** the call returns `FilterError::InvalidParams`
