# Tasks

## 1. Tests before implementation

- [x] 1.1 Update `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspectOnSquareRows` to assert document-shaped adjustment/fill masks beside square content cards, replacing the superseded square-mask expectation.
- [x] 1.2 Write `tst_canvas_mask_domain::allMaskCardsFollowDocumentAspect` for portrait/landscape documents, pixel/group/type/adjustment/solid/gradient/pattern owners, raster/vector masks, borders and hit targets.
- [x] 1.3 Write `tst_canvas_mask_domain::layerRowClickDefaultsToContent` for current/revisited rows after raster and vector activation, explicit mask clicks and unchanged history.
- [x] 1.4 Write `tst_canvas_mask_domain::adjustmentCreationActivatesMask` for the selected-new-adjustment/mask-active creation exception followed by plain-row content selection.

## 2. Implementation

- [x] 2.1 Share document-aspect geometry across mask cards and draw into those cards; preserve left content-card geometry and group folder slots.
- [x] 2.2 Reset plain real-row body clicks to content after dedicated controls have handled their events; preserve adjustment creation activation.

## 3. Verification and archive

- [x] 3.1 Compile after all source edits; run focused Layers/mask tests and the complete Qt suite. Finish verification serially with one job/process, keeping builds and tests separate.
- [x] 3.2 Sync/archive the change, run the full verification checks serially and strict OpenSpec/test-plan checks, and record results with no manual rows.
