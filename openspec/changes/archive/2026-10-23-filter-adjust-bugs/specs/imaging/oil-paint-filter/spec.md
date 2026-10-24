## ADDED Requirements

### Requirement: Oil Paint dialog exposes the engine parameter range

The Oil Paint dialog SHALL expose Shine as a `0..=10` control opening on
`5.5`, matching the engine's accepted `0.0..=10.0` range and the `shine / 10`
normalization both the CPU and GPU kernels apply. The slider SHALL reach the
maximum without clipping below it.

#### Scenario: Shine reaches the engine range [op_shine_range]

- **WHEN** the Oil Paint dialog opens and Shine is dragged to its maximum
- **THEN** the control's range is 0 through 10, its opening value is 5.5, and the committed parameter is 10
