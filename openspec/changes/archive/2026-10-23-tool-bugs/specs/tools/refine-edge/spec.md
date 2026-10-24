# Spec Delta

## ADDED Requirements

### Requirement: Refine Edge distances are document pixels

Boundary distances used by the Radius band and Shift Edge SHALL be in document
pixels. The 3-4 chamfer's orthogonal step SHALL be the divisor, so a pixel one
column from a straight edge reports 1 and a pixel four columns away reports 4.

#### Scenario: Distance is the pixel count [re_distance_pixels]

- **WHEN** a straight edge is measured at 1 and 4 columns away
- **THEN** the reported distances are 1 and 4, and a radius of 4 reaches only
  that far

### Requirement: Refine Edge preview scales with the proxy

The preview SHALL scale Radius, Smooth, and Feather by the preview proxy's
linear shrink factor before refining, so the proxy shows the edge width the
commit will apply. Contrast and Shift Edge are percentages and SHALL NOT scale.

#### Scenario: Preview parameters scale with the proxy [re_preview_scale]

- **WHEN** a large document is previewed at a reduced proxy size
- **THEN** Radius, Smooth, and Feather are scaled by the proxy's linear factor
  and Contrast and Shift Edge are unchanged

#### Scenario: Preview and commit agree within proxy tolerance [re_preview_commit_agree]

- **WHEN** the same edge is previewed at the proxy size and committed at document
  size with the same Radius
- **THEN** the preview's edge transition matches the committed edge transition
  scaled to the proxy within one proxy pixel
