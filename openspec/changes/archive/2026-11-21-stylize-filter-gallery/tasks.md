# Tasks

## 1. Catalogue ranks and tests

- [x] 1.1 Set the gallery ranks for Diffuse, Emboss, Extrude, Find Edges, Solarize, Tiles, Trace Contour, and Wind to 40–48, move Glowing Edges to 44, and shift Texture to 49–54
- [x] 1.2 Extend `tst_filter_gallery::categoriesFollowCs6` for the nine Stylize entries and add `tst_filter_gallery::stylizeFiltersFollowCs6`
- [x] 1.3 Render every Stylize filter in `tst_filter_gallery::thumbnailsRender`

## 2. Documentation and verification

- [x] 2.1 Update `ROADMAP.md` and mark FILT-5 shipped in `docs/dev/doc-code-parity-gaps.md` (docs commit needs `TASK-ALLOWS-DOCS`)
- [x] 2.2 Run the catalogue tests, the Filter Gallery Qt suite, `openspec validate --all --strict`, and the relevant fast gate
- [x] 2.3 Archive the change with `openspec archive stylize-filter-gallery` and verify `scripts/check-openspec-archived.sh`
