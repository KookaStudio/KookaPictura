# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Toggle and persist | Toggle rulers, save session, toggle again. | Rulers and saved setting follow the toggle. | tst_rulers_guides::rulersToggle |
| Ruler scale follows zoom | Calculate scales at specified zooms and units. | Major steps and subdivisions match the spec. | tst_rulers_guides::rulerScale |
| protractor_create | Draw an ordered line; Alt+Shift-drag from each endpoint, using known geometry. | The pressed endpoint is the vertex; primary order and D1 stay fixed, D2 and included angle have the expected values, the new arm is constrained, and history is unchanged. | tst_ruler_tool::protractorCreate |
| protractor_double | Double-click nearer each endpoint and at the midpoint, then drag. | The nearer endpoint is the vertex, with the end chosen on a tie; primary order remains fixed and D2 and included angle match known geometry. | tst_ruler_tool::protractorDouble |
| protractor_clear | Switch tools and clear. | Both arms hide, return and clear with D2. | tst_ruler_tool::protractorClear |
| ruler_straighten | Draw a tilted line, Straighten and Undo. | Canvas aligns and expands; one undo restores it. | tst_ruler_tool::straighten |
| ruler_straighten_noop | Try missing, aligned and refused rotations. | Pixels and history stay unchanged. | tst_ruler_tool::straightenNoop |
| ruler_origin_drag | Record ruler and Info X/Y readouts, drag the corner to a known point, pan, zoom, and change units. | Both axes and the ruler and Info X/Y readouts use the new origin and selected unit; history is unchanged. | tst_rulers_guides::originDrag |
| ruler_origin_unsnapped | Drag near visible guides, grid lines and slice edges. | Origin stays at the released document point without snapping. | tst_rulers_guides::originUnsnapped |
| ruler_origin_views | Change one view's origin and select another. | Each view retains its own origin. | tst_rulers_guides::originViews |
