# Test plan — Skew transform gestures

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Distort commits the dragged corner | Drag a Distort corner and commit | Corner lands at the pointer; one history state | `pictura-app::distort_moves_one_corner` |
| Perspective mirrors the opposite corner | Drag a Perspective corner | Opposite corner moves by the negated delta | `pictura-app::perspective_mirrors_the_opposite_corner` |
| `skew_corner` | Drag a Skew corner mostly horizontally, then mostly vertically | Only that corner moves along the chosen edge | `pictura-app::skew_corner_follows_its_dominant_edge`, `tst_transform_skew::cornerMovesAlongItsDominantEdge` |
| Skew slides one edge | Drag a Skew top edge diagonally and a left edge diagonally | Each edge moves only along its own axis | `pictura-app::skew_edge_slides_along_itself`, `tst_transform_skew::edgeSlidesOnlyAlongItself` |
| `skew_alt` | Drag a Skew corner and an edge with Alt held | The opposite corner/edge moves the other way | `pictura-app::skew_alt_mirrors_the_opposite_corner_or_edge`, `tst_transform_skew::altMirrorsTheOppositeEdge`, `tst_transform_skew::altMirrorsTheOppositeCorner` |
| `skew_type` | Drag a Skew corner on an editable type session | The followed edge slides; the quad stays a parallelogram and the layer stays type | `pictura-app::skew_keeps_editable_type_a_parallelogram`, `tst_type_tools::skewKeepsType` |
| `skew_convex` | Drag a Skew corner past the fold point | The stored quad stays convex | `pictura-app::skew_corner_stops_before_the_box_folds` |
| Escape cancels a projective mode exactly | Apply a projective gesture and cancel | Document and history are unchanged | `tst_transform_options::applyCommitsCancelRestores` |
| A degenerate drag is refused | Drag a Distort corner onto its neighbor | The quad is unchanged | `pictura-app::degenerate_gesture_is_refused` |
