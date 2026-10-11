## ADDED Requirements

### Requirement: Match Color adjustment

The system SHALL implement `Adjustment::MatchColor(MatchColorParams { target, source, neutralize, luminance, color_intensity, fade })` where `target` and `source` are Lab-space statistics (per-channel mean and standard deviation) and the command is reachable from `Image ▸ Adjustments ▸ Match Color…` only for RGB documents with an editable pixel layer active. For each pixel the system SHALL convert to CIELAB and transfer each channel by `(in − target.mean) · (source.std / target.std) + source.mean` (a flat target channel with zero standard deviation SHALL use a scale of 1; zeroed source statistics — no pixel contributed weight — SHALL leave the buffer bit-exactly unchanged), then multiply `L` by `luminance/100` and `a`/`b` by `(color_intensity − 1)/99`. When `neutralize` is set the source mean SHALL be replaced by `[source.mean_L, 0, 0]` so the transfer imposes no colour cast (a gray-world cast removal when the source statistics are the target's own). The result SHALL be blended toward the original pixel by `fade/100`, where fade 0 is the full match and 100 is an exact identity; statistics equal between source and target with default options SHALL be an exact no-op. Statistics SHALL be computed over a whole layer or document composite, each pixel weighted by its straight alpha and, when a selection plane is given, by that selection's coverage. `luminance` and `color_intensity` SHALL span `1..=200` and `fade` `0..=100`; out-of-range or non-finite values SHALL be rejected as `AdjustError::InvalidParams`. Alpha SHALL be preserved, and native-depth apply SHALL refuse the operation (it is dialog-only, with no PSD encoding). The transfer approximates Photoshop's closed kernel and is not claimed as parity.

#### Scenario: Statistics transfer moves the target toward the source

- **WHEN** Match Color is applied to a target with source statistics from a differently coloured image, neutralize off, luminance and colour intensity at 100, and fade 0
- **THEN** the target's per-channel Lab mean moves toward the source's within tolerance

#### Scenario: Neutralize removes the colour cast

- **WHEN** Match Color is applied with neutralize set
- **THEN** the output's mean `a` and `b` are zero (the mean is `[mean_L, 0, 0]`), regardless of the source's cast

#### Scenario: Colour intensity 1 is exactly gray

- **WHEN** Match Color is applied with colour intensity 1
- **THEN** every pixel's three RGB channels are equal within 1 LSB

#### Scenario: Fade extremes

- **WHEN** Match Color is applied with fade 100, or with the source statistics equal to the target's and default options
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Zeroed source statistics are an identity

- **WHEN** Match Color is applied with zeroed source statistics (an all-transparent source, or "Use Selection In Source" over an empty selection) against a non-flat target
- **THEN** the buffer is bit-exactly unchanged; the target is not collapsed onto the zeroed mean

#### Scenario: Selection-weighted statistics

- **WHEN** statistics are computed over a selection plane
- **THEN** only pixels with non-zero coverage contribute, weighted by their coverage and alpha

#### Scenario: Ignore selection applies to the whole layer

- **WHEN** Match Color is applied with a target selection active and "Ignore Selection When Applying Adjustment" checked
- **THEN** the whole layer is adjusted; with it unchecked, only the selected pixels change

#### Scenario: Invalid parameters are refused

- **WHEN** Match Color is applied with luminance, colour intensity, or fade outside its range or non-finite
- **THEN** the operation fails with `AdjustError::InvalidParams` and the buffer is unchanged, and alpha is never modified
