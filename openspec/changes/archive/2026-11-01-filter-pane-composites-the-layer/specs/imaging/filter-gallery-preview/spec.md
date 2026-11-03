## ADDED Requirements

### Requirement: Reduced copies scale vector masks

A reduced copy of the document SHALL map each vector-mask path through the copy's region and scale, and SHALL scale the mask's feather radius by the copy's scale, so a mask covers the same part of the picture in the copy as in the document.

#### Scenario: A reduced vector mask keeps its place [proxy_vector_mask]

- **WHEN** a layer with a vector mask is reduced, whole or as a cropped region
- **THEN** the mask's edge falls at its scaled position relative to the region

#### Scenario: A reduced vector mask scales its feather [proxy_vector_feather]

- **WHEN** a layer with a feathered vector mask is reduced
- **THEN** the feather radius is multiplied by the copy's scale

#### Scenario: A full-scale copy with a vector mask matches the document [proxy_full_scale]

- **WHEN** a Multiply layer with a vector mask is copied at scale 1 with no filter
- **THEN** the copy's composite equals the document's
