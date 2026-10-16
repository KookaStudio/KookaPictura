## ADDED Requirements

### Requirement: Sharpen tool

A Sharpen drag SHALL push each covered pixel of the active pixel layer away
from its 3×3 neighbourhood average by Strength times the tip's coverage,
restricted to the part of the pixel the Mode allows, keeping its alpha, and
SHALL work on what the previous dab left so that going over a spot again
sharpens it further. With Protect Detail on, a sharpened pixel SHALL stay
within the per-channel range its neighbourhood spans. A flat area and a
straight ramp SHALL be left unchanged. A stroke SHALL record exactly one
"Sharpen" history state when pixels changed, and a layer whose pixels are
locked SHALL be refused.

#### Scenario: Steepening a step

- **WHEN** the `tst_retouch_tools` Sharpen test drags along a grey step at Strength 100 % with Protect Detail off
- **THEN** one "Sharpen" state is recorded, the dark side darkens, the light side lightens, and pixels outside the tip are unchanged
