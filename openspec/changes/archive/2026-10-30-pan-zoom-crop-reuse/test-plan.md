# Test Plan

One row per spec scenario. `Coverage` names the automated test or `manual:` with
a reason. New tests are written before the implementation task that makes them
pass.

## ui/document-canvas

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| A pan repaint does not resample the document | Pan at a zoom that selects a stored level, then paint | The paint presents the zoom-selected level and never scales the full-resolution document | `tst_present_cache::panPresentsWithoutResamplingTheDocument` |
| `cvda_zero_copy_level` | Paint at a stored level below 0 | The paint draws the level's own pixels through a source rect, with no viewport copy | `tst_present_cache::storedLevelIsDrawnFromALevelView` |
| `cvda_level0_crop_reused` | Paint twice at level 0 and the same document rect | The second paint reuses the cached crop; the rebuild counter is unchanged | `tst_present_cache::level0CropIsReusedAcrossPaints` |
| `cvda_pan_reuses_crop` | Paint at level 0, pan without changing the zoom, paint again | The cached crop's overlap is reused, only the exposed band is cropped, and the presented pixels equal a fresh level-0 crop | `tst_present_cache::panAndZoomReuseTheLevel0Crop` |
| `cvda_zoom_reuses_crop` | Paint at level 0, zoom in within the level-0 band, paint again | The cached crop is reused (the new rect is inside it) and the presented pixels equal a fresh level-0 crop | `tst_present_cache::panAndZoomReuseTheLevel0Crop` |
| A zoom change invalidates the cache | Change the zoom across a level boundary and paint | The next paint chooses the new level and presents it | `tst_present_cache::zoomChangeSelectsTheNewLevel` |
| Cached present matches the transform draw | Paint at 100 % through the present path and through a direct transform draw | The two rendered images are pixel-identical | `tst_present_cache::level0PresentMatchesTheTransformDraw` |
| A coarse-level present is a single reduction | Paint at a zoom that selects a level below 0 | The source is that level's pixels; the full-resolution document is not rescaled | `tst_present_cache::storedLevelIsDrawnFromALevelView` |
| A document past the former cache bound still presents through a crop | Paint a document whose scaled present would exceed 64 megapixels at a high zoom | The paint presents the zoom-selected level and does not fall back to the transform draw | `tst_present_cache::largeDocumentPresentsThroughALevel` |

## Profile (evidence, not a pass/fail gate)

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| Pan-then-zoom frame times | On a 4000² document, paint at level 0, pan, then zoom in; time each `render` | The first zoom frame after a pan is not slower than a zoom from rest | `tst_present_cache::panThenZoomFrameTimes` |
| Level-0 viewport crop cost | Time a level-0 viewport crop against the exposed band the reuse path crops, at 4000² and 16000² | The level-0 crop is the per-frame cost the reuse path avoids | `pictura-app::scroll_zoom_pan_profile_4000`, `pictura-app::scroll_zoom_pan_profile_16000` |
| End-to-end manual check | Pan then zoom on a 4000²+ and a 16000² document | The first zoom step after a pan is smooth | manual: needs an interactive GUI session; headless render times stand in |
