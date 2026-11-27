## ADDED Requirements

### Requirement: The raw Exposure is applied in linear light before the clip

The raw open's develop SHALL take the Pictura Raw Exposure out of the
relative stage and add it to the look's `BaselineExposure` before
`develop_raw`, so it scales linear light before the develop clips. Pixels
that clip neither way SHALL develop as with Exposure in the relative stage,
within rounding, and highlights above the baseline's clip SHALL be
recoverable with a negative Exposure.

#### Scenario: Exposure matches the relative stage below the clip [raw_open_exposure_linear]

- **WHEN** a decoded source develops with Exposure +1 and −1 through the app's develop, and with the same Exposure folded into `BaselineExposure` through `develop_raw`
- **THEN** the samples are equal, and a mid-grey at Exposure +1 is brighter than at 0

#### Scenario: Negative Exposure recovers highlights [raw_open_exposure_recovers]

- **WHEN** a neutral source with `BaselineExposure` 2 and patches at linear 0.3 and 0.4 develops at Exposure 0 and −2
- **THEN** both patches are white at 0, and at −2 the brighter is below white and brighter than the other in every channel
