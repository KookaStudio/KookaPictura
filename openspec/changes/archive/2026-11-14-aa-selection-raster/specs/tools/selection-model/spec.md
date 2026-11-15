# Spec Delta

## ADDED Requirements

### Requirement: Anti-aliased primitive coverage
Primitive rasterisation SHALL support an anti-aliased mode for the selection
tools that expose it. When the mode is enabled, the new primitive SHALL be
rasterised with fractional boundary coverage in the range 0-255 before it is
feathered or combined: its interior SHALL remain 255, its exterior SHALL remain
0, and boundary pixels MAY hold intermediate values. When the mode is disabled,
every primitive coverage value SHALL be 0 or 255. Anti-aliasing SHALL NOT alter
the stored coverage of an existing selection except through the chosen combine
operation. The 50 %-coverage contour of an anti-aliased primitive SHALL follow
the analytic or polygonal primitive within 1 px. Coverage outside the canvas
SHALL be absent; a row or column clipped at the canvas edge SHALL keep its
clipped value. The Rectangular Marquee SHALL remain binary and SHALL NOT expose
an Anti-alias option.

#### Scenario: Anti-aliased edge has fractional coverage [sm_aa_fractional]
- **WHEN** an anti-aliased ellipse or polygon primitive is rasterised
- **THEN** some boundary pixels hold coverage strictly between 0 and 255, while interior pixels hold 255 and exterior pixels hold 0

#### Scenario: Anti-alias off is binary [sm_aa_off_binary]
- **WHEN** the same primitive is rasterised with Anti-alias off
- **THEN** every coverage value is 0 or 255

#### Scenario: The 50 % contour follows the primitive [sm_aa_contour]
- **WHEN** the 50 %-coverage contour of an anti-aliased primitive is extracted
- **THEN** it sits within 1 px of the analytic ellipse or the drawn polygon

#### Scenario: Anti-alias precedes feather and combine [sm_aa_order]
- **WHEN** an anti-aliased primitive is committed with Feather greater than zero or an Add, Subtract, or Intersect combine mode
- **THEN** the fractional primitive is feathered and then combined, so the result equals feathering the anti-aliased primitive before the combine operation rather than feathering a binary primitive

#### Scenario: Anti-alias does not touch an existing selection [sm_aa_existing]
- **WHEN** an anti-aliased primitive is built for a New, Add, Subtract, or Intersect commit
- **THEN** building the primitive does not change the stored coverage of the current selection; only the chosen combine operation changes it

#### Scenario: An anti-aliased primitive clips at the canvas edge [sm_aa_clip]
- **WHEN** an anti-aliased ellipse or polygon extends past the canvas
- **THEN** coverage outside the canvas is absent, the clipped row or column keeps its clipped value, and only the shape boundary inside the canvas is softened

#### Scenario: The rectangle stays binary [sm_aa_rect]
- **WHEN** the Rectangular Marquee is used
- **THEN** its mask contains only 0 and 255 and its options bar exposes no Anti-alias control
