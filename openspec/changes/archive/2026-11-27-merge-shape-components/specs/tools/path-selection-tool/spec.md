## ADDED Requirements

### Requirement: Merge Shape Components

Path Selection SHALL expose Merge Shape Components in its Path Operations menu. The command SHALL normalize overlapping components of the active editable path as one undoable edit. It SHALL compare the effective input and output fills on a shared i64 overlay grid and decline when component or hole counts, edge contacts, or bidirectionally certified boundary displacement differ beyond 1/256 document pixel. Overlay quantization and floating-point predicates limit detection of sub-grid topology changes. Work and saved paths SHALL use the 0.1-pixel adaptive flattening and recursion cap used by path coverage; layer vector masks SHALL use the renderer's closed polylines. It SHALL preserve independent contours and other document paths. An explicitly selected Paths row SHALL take priority over shape-layer fallback. It SHALL use the target's effective fill rule and preserve layer styling and mask flags. Unavailable, unchanged, or unsupported input SHALL leave document geometry and history, including redo, intact.

#### Scenario: Merge overlapping active Work Path contours [merge_work]

- **WHEN** a Work Path contains overlapping additive contours, an existing hole, and an independent curve, and the user chooses Merge Shape Components
- **THEN** the overlap is resolved into one outer contour, the hole and independent curve remain, and one Undo restores the original contours

#### Scenario: Merge nested overlapping contours [merge_nested]

- **WHEN** overlapping contours are enclosed by another contour that contributes to the target's winding or parity
- **THEN** the result preserves the original filled and unfilled regions, including the overlap, within the stated boundary tolerance

#### Scenario: Merge a saved path or a layer vector outline [merge_target]

- **WHEN** the user merges a selected saved path while a shape layer is selected, and then merges an eligible layer vector outline
- **THEN** each merge only changes its own target, the layer uses its own fill rule, and style and mask flags are retained

#### Scenario: Merge a curved layer mask [merge_curved_mask]

- **WHEN** overlapping curved contours in an eligible layer vector mask are merged
- **THEN** the result preserves the renderer's effective filled region within the stated boundary tolerance and retains any open mask contours unchanged

#### Scenario: Merge with a focused contour [merge_focus]

- **WHEN** one contour of the active path has focus and the path contains another overlapping group
- **THEN** the command merges the whole active path, including that other group, without changing another path

#### Scenario: No geometry to merge [merge_noop]

- **WHEN** the user has no editable target or tries to merge unchanged or unsupported geometry
- **THEN** no new history state or geometry change is produced and any redo branch remains available

#### Scenario: Decline an unsafe layer mask rewrite [merge_mask_declined]

- **WHEN** an imported mask has stored operations that cannot safely survive a rewrite, or normalization would remove its last filled contour and make the mask permissive
- **THEN** the mask bytes, effective fill and history, including any redo branch, remain unchanged
