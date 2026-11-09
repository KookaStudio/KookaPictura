## ADDED Requirements

### Requirement: Move preview applies the layer mask

While the Move tool drags a layer, the moving image SHALL show the layer as the
compositor draws it, including its layer mask and vector mask, so pixels the
mask hides MUST NOT appear during the drag. The preview MUST agree with the
committed move. The view preview of a layer too large to bake whole SHALL apply
the mask at the presented pyramid level as well. A layer whose channels alone
are its appearance SHALL keep the raw-channel moving image.

#### Scenario: A masked layer's drag hides the masked pixels [lct_move_preview_mask]

- **WHEN** a layer with a layer mask that hides part of its content is dragged
  with the Move tool
- **THEN** the moving image leaves the hidden part transparent and the drag
  preview matches the committed move

#### Scenario: A large masked layer's view preview applies the mask [lct_move_preview_mask_view]

- **WHEN** a layer whose content is larger than 4 MP carries a layer mask and is
  dragged while the canvas presents a pyramid level
- **THEN** the reduced moving image applies the mask at that level
