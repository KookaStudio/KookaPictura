# Spec Delta

## ADDED Requirements

### Requirement: Liquify processes only an active selection's bounding rectangle

When a selection is active, the system SHALL confine the Liquify session, its
preview, and its commit to the selection's bounding rectangle intersected with
the active layer, expressed in layer coordinates. The dialog SHALL show only
that rectangle, opened at 100 % and centred on it. Every pixel outside it MUST
stay bit-identical in the preview and after the commit. A brush stroke or mesh
node outside it SHALL change nothing. The selection's shape MUST NOT restrict
processing inside the rectangle: a pixel inside it but outside a lasso or
ellipse SHALL be processed. Without a selection the whole layer SHALL be
processed. An empty selection, or one whose bounding rectangle does not
intersect the layer, SHALL refuse to open the dialog. A mesh SHALL carry its
rectangle, so a kept mesh (Load Last Mesh, Last Filter) keeps the confinement
and a mesh stretched to another layer size scales it with the mesh.

#### Scenario: A rectangular selection confines the preview and the commit [lq_selection_rect]

- **WHEN** a rectangular selection is active and a stroke is drawn outside its
  bounding rectangle
- **THEN** the preview shows only that rectangle, the stroke changes no pixel,
  and OK records no history state

#### Scenario: A stroke inside the rectangle changes only inside it [lq_selection_rect_inside]

- **WHEN** a stroke inside the bounding rectangle is committed
- **THEN** pixels inside the rectangle change and every pixel outside it is
  unchanged

#### Scenario: A non-rectangular selection uses its bounding rectangle [lq_selection_bbox]

- **WHEN** an elliptical selection is active and a stroke inside its bounding
  rectangle but outside the ellipse is committed
- **THEN** the pixels under the stroke change and every pixel outside the
  rectangle is unchanged

#### Scenario: A selection that misses the layer refuses the dialog [lq_selection_miss]

- **WHEN** the selection's bounding rectangle does not intersect the active
  layer, or the selection is empty
- **THEN** Liquify does not open and records no history state
