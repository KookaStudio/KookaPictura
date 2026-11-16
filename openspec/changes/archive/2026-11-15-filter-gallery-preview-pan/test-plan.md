# Test Plan

One row per scenario in the delta spec. Every row is automated.

## imaging/filter-gallery-preview

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A large picture previews at pane resolution | Open the gallery on a 4000×3000 document in a 900×600 dialog. | The preview image is narrower than 1000 device pixels and keeps the 4:3 aspect. | `tst_filter_gallery::largePicturePreviewsReduced` |
| Previewing leaves the document alone | Preview a visible effect, then cancel. | The canvas is pixel-identical and the history is unchanged. | `tst_filter_gallery::stackPreviewsInThePaneOnly` |
| Full-scale copy matches the commit | Filter a whole-document copy at scale 1. | It equals the composite with the same filter applied to the layer. | `pictura-render::full_scale_proxy_renders_like_the_document` |
| fgp_pixel_scale | Preview a Gaussian Blur stack on a quarter-size copy. | The blur radius scales with the copy; other controls keep their values. | `tst_filter_gallery::previewScalesPixelParams` |
| fgp_thumbnail_scale | Render the Mosaic Tiles catalog thumbnail. | Opening pixel distances are scaled, rounded, and held to range. | `tst_filter_gallery::thumbnailScalesPixelParams` |
| proxy_vector_mask | Reduce a layer with a vector mask, whole and cropped. | The mask edge falls at its scaled position relative to the region. | `pictura-render::reduced_vector_mask_keeps_its_place` |
| proxy_vector_feather | Reduce a feathered vector mask. | The feather radius is multiplied by the copy's scale. | `pictura-render::reduced_vector_mask_scales_its_feather` |
| proxy_full_scale | Copy a Multiply layer with a vector mask at scale 1 with no filter. | The copy's composite equals the document's. | `pictura-render::full_scale_proxy_keeps_a_masked_multiply_layer` |
| fgp_pan_reuse | Open at 200%, wait for the preview, scroll less than the pan margin, then wait past the debounce. | The cached region still covers the pane; the image and region are unchanged. | `tst_filter_gallery::panReusesTheCachedPreview` |
| fgp_pan_refilter | Open at 200% and scroll past the cached pan margin. | A new filtered copy covering the pane is rendered after the debounce. | `tst_filter_gallery::panPastTheMarginRefilters` |
| fgp_pan_invalidate | Change the zoom after a cached preview; change the stack. | The preview re-filters at the new scale or stack. | `tst_filter_gallery::zoomChangeRefiltersTheCache`, `tst_filter_gallery::stackPreviewsInThePaneOnly` |
| fgp_pan_commit | Pan the preview, then press OK. | The full-resolution layer receives the same visible stack as without panning. | `tst_filter_gallery::okCommitsOneHistoryState` |
