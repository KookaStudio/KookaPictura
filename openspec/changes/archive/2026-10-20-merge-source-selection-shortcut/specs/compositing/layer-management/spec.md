# Spec Delta

## ADDED Requirements

### Requirement: Adjustment and fill layers as merge sources

The system SHALL allow an adjustment or fill-content layer to be an input to
Merge Down and Merge Layers, and SHALL bake its effect into the resulting pixel
node through the document compositor. The merge target, meaning the lower layer
for Merge Down and the bottommost selected layer for Merge Layers, MUST remain a
raster or group layer that is not an adjustment or fill-content layer. A refused
merge MUST leave the document unchanged and MUST NOT add a history state.

#### Scenario: An adjustment source merges down

- **WHEN** an adjustment layer sits directly above a pixel layer and Merge Down
  runs with the adjustment layer selected
- **THEN** the two layers are replaced by one pixel layer whose pixels equal the
  compositor's composite of the pair, whose adjustment data is gone, and whose
  name, blend mode, and opacity come from the lower layer

#### Scenario: An adjustment above the bottom of a selection merges

- **WHEN** the selection contains an adjustment layer and at least one pixel
  layer below it, and the bottommost selected layer is a pixel layer, and Merge
  Layers runs
- **THEN** the selected layers are replaced by one pixel layer with the
  adjustment baked into the selected layers below it

#### Scenario: An adjustment target is still refused

- **WHEN** the merge target is an adjustment layer and merge runs
- **THEN** the merge is refused, the document is unchanged, and no history state
  is added
