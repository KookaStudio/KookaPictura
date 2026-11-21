# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Categories follow CS6 | Open the Filter Gallery and inspect the Stylize category. | It contains Diffuse, Emboss, Extrude, Find Edges, Glowing Edges, Solarize, Tiles, Trace Contour, and Wind in that order; the other category counts are unchanged. | `tst_filter_gallery::categoriesFollowCs6`, `tst_filter_gallery::stylizeFiltersFollowCs6` |
| Thumbnails show the picture filtered | Open the gallery on a textured picture and wait for the Stylize thumbnails. | Every Stylize thumbnail renders the picture through its filter instead of falling back to the unfiltered sample. | `tst_filter_gallery::thumbnailsRender` |
