## ADDED Requirements

### Requirement: Stack Images selection and tone

Stack Images SHALL leave flat and out-of-focus areas to the lowest layer covering
them, and only blown or crushed areas SHALL lose on exposure. An upper layer with
real detail SHALL win unless clearly less sharp. With Seamless on, broad tone SHALL
come from the lowest covering layer and detail from the sharpest, and gains SHALL
correct colour without changing overall brightness. Gains SHALL be measured over
flat areas. Panorama SHALL still match exposure. This is an approximation tuned
against observed Photoshop output.

#### Scenario: A flat upper background is dropped [abl_flat_base]

- **WHEN** a layer with a detailed subject on a flat background is stacked over a bright flat base
- **THEN** its mask hides the flat background and keeps the subject

#### Scenario: Upper detail survives busy texture [abl_upper_detail]

- **WHEN** a detailed subject is stacked over fine busy texture
- **THEN** the subject's mask reveals it

#### Scenario: Tone follows the base [abl_base_tone]

- **WHEN** a dark subject is stacked over a bright base with Seamless on
- **THEN** the merged subject's broad tone follows the base and its detail survives
