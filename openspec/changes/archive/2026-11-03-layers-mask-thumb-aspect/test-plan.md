# Test Plan

Covers every scenario in the `ui/layers-panel` delta for
`layers-mask-thumb-aspect`. The change fixes the raster mask thumbnail and pins
the already-correct sub-canvas layer image thumbnail; no manual rows are
expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lpr_thumb_aspect` | Show a regular layer of a document whose width and height differ in the Layers panel | The drawn card's aspect ratio matches the document, with the unused slot area left empty | `tst_layers_panel_fixes::thumbnailFillsRow`, `tst_layers_panel_fixes::tallThumbnailKeepsDocumentAspect` |
| `lpr_thumb_source_aspect` | Show a full-width layer on a tall document with Entire Document contents | The thumbnail image carries the document aspect and its rightmost column is layer content, not transparent padding | `pictura-app::positioned_thumbnail_follows_document_aspect`, `tst_layers_panel_fixes::tallThumbnailKeepsDocumentAspect` |
| `lpr_thumb_square_card` | Show an adjustment, fill, type, or content-less synthetic row on a non-square document | Its card is 1:1 while a pixel layer's thumbnail is still letterboxed to the document | `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline`, `tst_layers_panel_fixes::fillThumbnailIsSquareWithOutline`, `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline`, `tst_layers_snapshot::fillThumbnailIsSquare`, `tst_layers_smart_filters::smartFiltersRowHasNoOutline` |
| `lpr_thumb_sub_canvas_layer` | Create a 20x10 document, paste a 4x4 raster layer at (6,2), snapshot with `entireDocument=true`; separately paint the panel for a 400x100 document with a 50x50 layer at (50,25) | The snapshot thumbnail is (24,12) with opaque content at (7,2) and transparent (0,0); the painted image-card margin at (+1,+1) is the checker white (255) | `tst_layers_snapshot::subCanvasLayerPlacement`, `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspect` |
| `lpr_thumb_mask_aspect` | Create a 400x100 document, paste a 50x50 raster layer at (50,25), add a layer mask, snapshot and paint with Entire Document; repeat the snapshot with `entireDocument=false` | Entire Document: the mask thumbnail is (24,6), opaque at (3,2), transparent at (0,0), the card is (24,6), and the painted mask-card margin at (+1,+1) is checker white (255). A mask wider than the canvas is clipped, not scaled. Layer Bounds: the mask image is (24,24); the card keeps the existing document-aspect letterbox (covered by `lpr_thumb_aspect`) | `pictura-app::positioned_mask_thumbnail_follows_document_aspect`, `tst_layers_snapshot::subCanvasLayerPlacement`, `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspect` |
