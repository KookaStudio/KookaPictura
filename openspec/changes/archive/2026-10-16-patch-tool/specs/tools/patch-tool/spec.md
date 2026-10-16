## ADDED Requirements

### Requirement: Patch engine

`pictura_paint::healing::patch_layer` SHALL heal one pixel layer through a
document-sized selection coverage mask and a drag offset. In Source mode it
SHALL repair the selection with a Poisson solve sampled from the selection
displaced by the drag; in Destination mode it SHALL apply the selection's
shape at the drag target, sampling back from the selection; Transparent SHALL
keep the patched area's own colour; Content-Aware SHALL rebuild the selection
from its surroundings and ignore the drag. An empty selection, a mask of the
wrong size, or a zero drag outside Content-Aware SHALL be a no-op, and a
pixel-locked or non-raster layer SHALL be refused. The result SHALL be
deterministic.

#### Scenario: Source mode repairs the selection

- **WHEN** `patch_layer` runs in Source mode over a dark blot with a drag onto a light field
- **THEN** the blot becomes light and the sampled area is unchanged

#### Scenario: Destination mode patches the drag target

- **WHEN** the same selection and drag run in Destination mode
- **THEN** the pixels at the drag target change and the selection's own pixels do not

#### Scenario: Content-Aware ignores the drag

- **WHEN** `patch_layer` runs Content-Aware twice with different drags
- **THEN** both results are identical

### Requirement: Patch tool

The Patch tool SHALL trace a freehand selection outline when dragged from
outside the selection (combined with any existing selection by the options
bar's mode), and SHALL patch through the selection when dragged from inside
it, previewing the outline at the drag offset and recording exactly one
"Patch Tool" history state on release. A release without movement SHALL record
nothing. The options bar SHALL offer Patch (Normal / Content-Aware), Source /
Destination, and Transparent, disabling the last three under Content-Aware.

#### Scenario: Outline then drag

- **WHEN** the `patch_tool` self-test outlines a black blemish on white and drags the outline onto clean pixels
- **THEN** one "Patch Tool" state is recorded, the blemish pixel is near white, and the sampled pixel is unchanged

#### Scenario: Content-Aware disables sampling controls

- **WHEN** the Patch mode is set to Content-Aware
- **THEN** the Source, Destination, and Transparent controls are disabled
