# Test Plan

One row per scenario in the change's spec deltas. Existing Place sequencing and
cancel coverage is reused from `tst_place_sequence`; the new fit and
non-destructive scale cases use `tst_place_fit` and the `pictura-render` engine
tests.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| A PSD source is placed | Append a valid PSD through `place_smart_object` | Topmost layer, no existing layer changed | `pictura-render::place_appends_topmost_channel_less_embedded_object` |
| Malformed bytes are refused | Call `place_smart_object` with non-PSD bytes | `None`, document unchanged | `pictura-render::place_refuses_malformed_source` |
| The layer geometry and metadata are stamped | Append a PSD of size w×h | Native rect, no channels, embedded payload/name/filetype/creator | `pictura-render::place_appends_topmost_channel_less_embedded_object` |
| The composite shows the placed source | Place a solid PSD and composite | Source colour inside the placed rect | `pictura-render::placed_source_renders_inside_its_rect` |
| Save then load preserves the placed object | Write and read a document with a placed layer | Embedded payload survives | `pictura-render::placed_layer_round_trips` |
| place_fit_landscape | Place a 4000 × 3000 PSD into 1000 × 1000 | Rect 1000 × 750 centred; payload/native dimensions retained | `pictura-render::fit_smart_object_scales_an_oversized_source`, `tst_place_fit::fittedPsdStartsAtTheFittedRect` |
| place_fit_portrait | Place a 3000 × 4000 PSD into 1000 × 800 | Rect 600 × 800 centred; uniform scale | `pictura-render::fit_smart_object_centres_a_portrait_source` |
| place_fit_native | Place a 640 × 480 source into 1000 × 1000 | Native rect; no upscale | `pictura-render::fit_smart_object_does_not_upscale`, `tst_place_fit::sourceThatFitsIsNotUpscaled` |
| place_fit_raster_native | Place a 4000 × 3000 raster into 1000 × 1000 | Native embedded source; fitted rect; no downscaled source | `tst_place_fit::fittedRasterKeepsItsNativeSource` |
| place_fit_enlarge | Fit a 4000 × 3000 source, scale the Place session 2×, commit | Rect 2000 × 1500; smart object/payload/native dimensions remain; clipped | `tst_place_fit::fittedPlaceEnlargesWithoutLosingTheSource`, `pictura-render::transform_smart_object_rect_keeps_the_source` |
| place_fit_state | Place oversized, commit the session unchanged | Exactly one `"Place"` state; no fit state | `tst_place_fit::fitIsPartOfThePlaceState` |
| A supported image is placed as a smart object | Place a supported raster and inspect the layer | Layer path returned; embedded smart object; one `"Place"` state | `tst_place_sequence::multiSelectPlacesOneLayerPerFile` |
| The placed layer renders from its embedded native source | Place a solid-color raster | Composite shows the pixels from the embedded source; no raster proxy | `tst_place_fit::fittedRasterKeepsItsNativeSource` |
| place_image_alpha | Place a transparent/opaque raster, then Reset and Rasterize | Transparent and opaque alpha values survive | `pictura-render::convert_to_smart_object_keeps_merged_transparency`, `tst_place_fit::transparentRasterKeepsAlphaThroughResetAndRasterize` |
| A refusal records nothing | Place a missing or undecodable raster | Empty path, no history state, document unchanged | `tst_place_sequence::failedFileDoesNotStopTheRestAndIsSummarized` |
| PSD stays native for Place | Place a PSD | Routed through `place_smart_object`, not Qt | `tst_place_sequence::mixedFormatsPlacePerFile` |
| Success records one state and returns the layer path | Choose a PSD in `File > Place…` and commit | One `"Place"` state and one layer | `tst_place_sequence::mixedFormatsPlacePerFile` |
| A raster image records one state and returns the layer path | Choose a PNG and commit | One state and one smart-object layer | `tst_place_sequence::multiSelectPlacesOneLayerPerFile` |
| A successful place starts a transform session | Place a supported image | Session active; `"Place"` remains | `tst_place_sequence::eachPlaceWaitsForThePreviousTransform` |
| Refusal records nothing | Select an undecodable raster among good files | Failed file adds nothing; good files place | `tst_place_sequence::failedFileDoesNotStopTheRestAndIsSummarized` |
| Enabled only when a document is open | Dispatch without a document | Action disabled; no dialog | `tst_place_sequence::placeUnavailableWithoutDocument` |
| place_dialog_filters | Open `File > Place…` | Images filter beside Photoshop entries; multi-select | `tst_place_sequence::placeDialogOffersImagesFilter` |
| place_multi | Select two files; commit each | Two layers, two states, dialog order | `tst_place_sequence::multiSelectPlacesOneLayerPerFile` |
| place_multi_mixed | Select a PSD and a PNG | PSD through `place_smart_object`; raster through `place_image` | `tst_place_sequence::mixedFormatsPlacePerFile` |
| place_multi_partial | Good, undecodable, good | Two layers/states; one summary names the failure | `tst_place_sequence::failedFileDoesNotStopTheRestAndIsSummarized` |
| place_multi_all_failed | Select only invalid files | No layer/state; one summary | `tst_place_sequence::allFilesFailShowsOneSummary` |
| place_multi_cancelled | Cancel a mismatched raster's profile warning | First records nothing; remainder places; no summary entry | `tst_place_sequence::cancelledProfileWarningSkipsWithoutSummary` |
| place_multi_profile_policy | Fail a raster after accepting a profile, then place another | Second uses its own/default policy | `tst_place_sequence::failedPlaceDoesNotReuseTheProfilePolicy` |
| place_multi_no_document | No document or active transform | Disabled; no dialog/placement | `tst_place_sequence::placeUnavailableWithoutDocument`, `tst_place_sequence::placeUnavailableDuringTransform` |
| A channel-less embedded object is transformed by the raster op | `transform_layer` on a channel-less smart object | Channels materialize; smart object dropped | `pictura-render::channel_less_embedded_object_materializes_and_consumes` |
| An undecodable channel-less target is refused | `transform_layer` with an undecodable payload | False, document unchanged | `pictura-render::undecodable_channel_less_target_is_refused_untouched` |
| Menu Place enters the session | Place through the menu command | Session starts; `"Place"` remains | `tst_place_sequence::eachPlaceWaitsForThePreviousTransform` |
| ft_place_fit_start | Place oversized and inspect the session | Session rect is the fitted rect; payload/native source remain | `tst_place_fit::fittedPsdStartsAtTheFittedRect` |
| ft_place_fit_native | Place a source that fits | Session starts at the native rect | `tst_place_fit::sourceThatFitsIsNotUpscaled` |
| ft_place_fit_enlarge | Enlarge a fitted placement and commit | One `"Free Transform"` state; source remains; clipped | `tst_place_fit::fittedPlaceEnlargesWithoutLosingTheSource` |
| Canvas drop enters the session | Drop files on the canvas | One session on the last placed layer; states remain | manual: self-test codes 291/292 cover the drop sequencing and session entry |
| ft_canvas_drop_fit | Drop oversized files on the canvas | Every layer fitted; last session starts fitted; sources remain | manual: the drop router uses the same Place bridges; no direct Qt harness for the drop fit |
| Cancelling keeps the placement | Cancel a canvas-drop session | Layer remains at its fitted rect; state remains | manual: self-test code 292 covers the single-file drop cancel; the multi-file target is not asserted |
| ft_menu_multi_place | Place two files; commit the first | Second waits for the first session; then starts | `tst_place_sequence::eachPlaceWaitsForThePreviousTransform`, `tst_place_sequence::applyControlCommitsAndAdvances` |
| ft_menu_place_escape | Escape/Cancel a placement | Layer and `"Place"` state removed | `tst_place_sequence::escRemovesThePlacementAndHistory`, `tst_place_sequence::cancelControlRemovesThePlacement` |
| ft_menu_place_escape_next | Escape first; choose Continue | First removed; second placed | `tst_place_sequence::escContinuePlacesTheNextFile` |
| ft_menu_place_escape_last | Escape the last item | Removed; no prompt | `tst_place_sequence::escOnLastItemEndsWithoutDialog` |
| ft_menu_place_escape_whole | Escape first; cancel the whole Place | No further placements; no summary | `tst_place_sequence::escCancelWholeStopsTheBatch` |
| ft_tool_switch_dont_apply | Switch tool; Don't Apply | Layer/state remain; next file places | `tst_place_sequence::toolSwitchDontApplyKeepsThePlacement` |
| ft_menu_place_identity_commit | Commit unchanged | No `"Free Transform"` state; next places | `tst_place_sequence::identityCommitAdvancesWithoutHistory` |
| ft_menu_place_document_switch | Activate another document mid-session | Batch stops; placed layers remain; no summary | `tst_place_sequence::documentSwitchAbortsTheBatch` |
| ft_menu_place_document_close | Close the document mid-session | Batch stops; no summary | `tst_place_sequence::closingDocumentAbortsTheBatch` |
