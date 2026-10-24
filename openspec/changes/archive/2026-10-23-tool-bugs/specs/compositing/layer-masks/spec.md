# Spec Delta

## ADDED Requirements

### Requirement: Healing refuses a mask target

With a mask edit target active, the Spot Healing Brush and Healing Brush SHALL
refuse to start with a visible message, recording no history state and leaving
both the layer's pixels and the mask's coverage unchanged.

#### Scenario: Healing refuses a mask target [lm_healing_refuses_mask]

- **WHEN** a mask edit target is active and a Spot Healing or Healing Brush
  press starts
- **THEN** the press is refused with a visible message, no history state is
  recorded, and neither the layer's RGB nor the mask coverage changes
