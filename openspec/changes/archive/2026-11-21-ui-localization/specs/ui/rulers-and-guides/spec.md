# Rulers and Guides Spec Delta

## ADDED Requirements

### Requirement: Ruler labels use the system numeric locale

Ruler tick labels and displayed ruler-based measurements SHALL use the system
numeric locale independently of the UI language. Locale-aware formatting SHALL
preserve the existing units, precision, and mapping between ticks and document
positions. Calibrated ruler length in the options bar SHALL also use the
system numeric locale while preserving its unit and canonical numeric value.

#### Scenario: Calibrated ruler length uses the numeric locale [loc_calibrated_ruler]

- **WHEN** a calibrated ruler length is fractional under a comma-decimal locale
- **THEN** the options bar displays a comma decimal with the existing unit and
  precision, while the underlying calibrated value remains numeric and unchanged

#### Scenario: Ruler values use a comma decimal [loc_ruler_values]

- **WHEN** a ruler tick and a displayed ruler-based measurement each show a
  fractional value under a comma-decimal numeric locale
- **THEN** their numbers use a comma, retain the existing units and precision,
  and represent the same document positions as under a period-decimal locale
