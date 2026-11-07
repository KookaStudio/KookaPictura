# Layers panel delta

## ADDED Requirements

### Requirement: Active layer has a red Quick Mask highlight

While Quick Mask is active, the active layer's row SHALL use a red highlight in
place of its ordinary brighter gray active highlight. The highlight SHALL
follow the active layer and active document, and SHALL disappear when no layer
is selected. Leaving Quick Mask SHALL restore ordinary row highlighting. This
mode indication SHALL NOT change layer selection, a layer's color label,
document pixels, or history.

#### Scenario: Quick Mask replaces the active highlight [lp_quick_mask_highlight]
- **WHEN** Quick Mask is entered and then left with a layer active
- **THEN** that row changes from its normal active highlight to red and back, and its stored color label and history remain unchanged

#### Scenario: The red highlight follows the active target [lp_quick_mask_target]
- **WHEN** the active layer changes, all layers are deselected, or a different document is activated while Quick Mask is active
- **THEN** the red highlight tracks only the newly active layer in a Quick Mask document, with no stale red highlight on the previous target or an empty selection
