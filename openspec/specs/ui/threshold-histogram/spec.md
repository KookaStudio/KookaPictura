# ui/threshold-histogram Specification

## Purpose
Show the luminance distribution of the Threshold adjustment's input so the user can choose a cutoff.

## Requirements

### Requirement: Threshold input histogram

The Image Adjustments dialog and adjustment-layer Properties SHALL show a 256-bin luminance histogram between the Threshold Level value row and slider. The dialog SHALL use the opening edit target, including editable grayscale for a raster mask or Quick Mask when targeted (using Quick Mask `working_value` in its current polarity), and SHALL keep the distribution stable while previewing. Properties SHALL use the source beneath the Threshold layer. Properties SHALL refresh the distribution when the lower source or current selection changes outside a slider gesture. Sampling SHALL respect the current selection coverage and skip fully transparent color samples. An empty source SHALL remain empty in the chart and leave the level control available.

#### Scenario: Dialog shows the selected opening input [thhist_dialog]
- **WHEN** Threshold opens on an active pixel layer with a selection
- **THEN** its graph shows the selected input tones above the level slider and keeps them during level and live preview changes

#### Scenario: Mask and feathered selection contribute their stored values [thhist_mask]
- **WHEN** a raster mask or Quick Mask is the destructive adjustment target and selection coverage varies
- **THEN** the graph uses its stored grayscale values weighted by selection coverage

#### Scenario: Properties shows the source below the adjustment [thhist_props]
- **WHEN** the Threshold layer is selected in Properties
- **THEN** its graph uses lower source tones and excludes the adjustment's own result and layers above it

#### Scenario: Properties refreshes its source and selection [thhist_refresh]
- **WHEN** the lower source or current selection changes outside a Threshold slider gesture
- **THEN** Properties refreshes the histogram to reflect the new source and selection

#### Scenario: Properties tracks a selection move [thhist_selection_move]
- **WHEN** the selection is moved over different lower source tones and then cancelled
- **THEN** Properties refreshes for the translated selection during the move and restores the original selection's histogram on cancellation

#### Scenario: Properties respects group boundaries [thhist_groups]
- **WHEN** a Threshold layer is inside a pass-through group or an isolated group
- **THEN** its graph includes contributing lower pass-through context and stops at an isolated group boundary

#### Scenario: A small selection on a large source is sampled [thhist_large_selection]
- **WHEN** a small off-origin zero-exterior selection is present on a large source
- **THEN** deterministic bounded tile sampling uses its cached mask bounds and represents its selected tones

#### Scenario: Fully transparent color input has an empty chart [thhist_empty]
- **WHEN** the Threshold dialog targets a fully transparent pixel layer
- **THEN** the graph has no occupied bins and the Threshold Level slider remains available
