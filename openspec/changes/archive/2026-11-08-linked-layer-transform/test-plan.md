# Test plan

Every scenario in the delta specs has a row. The new linked-set rows drive the
real Free Transform bridge and Qt canvas; the existing lifecycle rows point at
the suites that already cover them.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Begin, gesture, commit records one state | Begin on a raster layer, scale, commit | One `Free Transform` state and the rect follows | `tst_transform_controls::handleDragScalesAndCommits` |
| Escape restores the document exactly | Begin, rotate, cancel | Document equals its pre-session value | `tst_transform_options::applyCommitsCancelRestores` |
| An identity commit records nothing | Begin and commit without a gesture | No history state | `tst_transform_controls::idleBoxLeavesTheLayerAlone` |
| Beginning on the active path is a no-op | Begin twice on the same path | The live session is unchanged | manual: the existing begin-path tests cover the unchanged single-target no-op |
| Beginning on a different path restarts the session | Begin on one path, then another | The first is cancelled, the second begins | `tst_document_session::replaceEntryPointsCancelTheSession` |
| Switching documents cancels the session | Begin, then switch the active document | The session ends without mutating the original | manual: document switching is covered by the document-session close path, unchanged here |
| `ftr_linked_rotate` | Link two squares, begin on one, rotate 90°, commit, then undo | Both rotate about the union centre, one state, link survives, undo restores both | `tst_linked_layers::linkedFreeTransformRotatesTheSet` |
| `ftr_linked_hidden` | Link two squares, hide one, rotate the other, commit | The hidden partner's content receives the same transform | `tst_linked_layers::hiddenLinkedMemberTransformsToo`, `pictura-app::a_linked_transform_includes_a_hidden_partner` |
| `ftr_linked_locked` | Link a position-locked layer to another, begin a transform | The locked member is not a session target | `pictura-app::a_linked_transform_skips_a_position_locked_partner` |
| `ftr_linked_projective` | Link two squares, begin a Skew session, set a shear, commit | Both layers shear and stay linked | `tst_linked_layers::linkedSkewTransformsTheSet` |
| `ftr_linked_warp` | Link two layers, enter Warp, drag the mesh, commit | Preview and commit apply the same mesh to every member | manual: the linked-set harness drives the transform bridge; no Qt test drives a warp mesh on a link set yet |
