# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vpr_format` | Encode and decode a tilted root (grid 40), a related plane at 60°, a red root, and an empty set | Each decodes equal, red stays red | `pictura-filters::planes_round_trip_through_bytes` |
| `vpr_refuse` | Decode cut, overlong, version 2, unknown kind, NaN, late parent, edge 4 | Each refused | `pictura-filters::malformed_plane_bytes_are_refused` |
| `vpr_resource` | Write, rewrite and remove the planes beside another plug-in's id-4900 block, XMP and trailing bytes | Others untouched, rewrite in place, removal restores the section | `pictura-codec::the_planes_resource_is_replaced_in_place_and_others_are_kept` |
| `vpr_photoshop` | Open a PSD, TIFF and JPEG saved with planes in CS6 | Opens without a warning, pixels intact | manual: needs Photoshop CS6, which CI does not have |
| `vpr_jpeg` | Save IPTC and 100 001 bytes of planes to a JPEG; read a plain JPEG | Two APP13 segments, identical block; plain has none | `pictura-codec::the_photoshop_block_round_trips_and_splits_across_segments` |
| `vpr_tiff` | Save planes to a TIFF, then re-save | Identical block, one tag 34377 | `pictura-codec::the_photoshop_block_round_trips` |
| `vpr_open` | Open a JPEG block with ICC + planes + IPTC, a TIFF, a PNG | Profile dropped; TIFF block read; PNG none | `pictura-codec::raster_open_reads_the_block_without_its_profile` |
| `vp_files_reopen` | Two planes, Grid Size 40; save PSD, PSB, TIFF (with Qt's TIFF writer), JPEG, PNG; reopen each | Unmodified after save; planes back except PNG | `tst_vanishing_point_files::planesReopenFromSavedFiles` |
| `vp_files_modified` | Cancel; OK unchanged; OK a new plane; save; OK unchanged; delete and OK; save; reopen | Modified only after the draw and the delete, no state; no planes reopen | `tst_vanishing_point_files::planeOnlyOkMarksTheDocumentModified` |
| `vp_files_fit` | Planes for 100×80, empty, and a 120×80 document | Round trip; no block; neither read nor written | `pictura-app::saved_planes_must_fit_the_canvas` |
