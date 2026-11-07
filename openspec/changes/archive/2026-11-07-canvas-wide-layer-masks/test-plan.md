# Test plan

This is a planning artifact. References below name tests to add or extend in the
apply phase; they are not claims of existing coverage or successful execution.
Every row is automatable. There are no required manual reviewer rows and no
Photoshop parity claim. Use generated in-memory/temporary PSD/PSB fixtures.

The new `tst_canvas_mask_domain` suite keeps new cases out of the already large
`tst_mask_edit.cpp`. Register it in the Qt tests CMake file and shard independent
cases using the existing harness. Rust mask cases belong in a sibling
`tests.rs`/test module under the mask concern if inline files approach their cap;
codec round trips belong in a new `tests/offcanvas_mask.rs` integration suite.

## Mask creation and adapters

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Reveal All creates a full mask | Create a 3x2 offset opaque layer in a 12x8 canvas; add Reveal All; sample the complete canvas and exterior. | White inside content, black elsewhere/default, canvas-covered storage, identical composite. | `pictura-render::mask_creation_covers_canvas` |
| Hide All creates an empty mask | Repeat the offset fixture with Hide All. | Canvas and exterior are zero; owning layer is hidden. | `pictura-render::mask_creation_covers_canvas` |
| `lpr_mask_add_target` | Add a mask through the active-layer bridge and paint a known canvas point. | New mask becomes the target; coverage changes, pixels do not. | `tst_canvas_mask_domain::createdMaskBecomesTarget` |
| A second mask is refused | Add a mask twice, retaining the original state and history index. | Second add returns failure without changing mask, target, or history. | `pictura-render::mask_creation_refusals_are_atomic`, `tst_canvas_mask_domain::canvasMaskRefusalIsAtomic` |
| An unknown path is refused | Attempt creation at an invalid path. | Failure and exact pre-operation document. | `pictura-render::mask_creation_refusals_are_atomic` |
| `lm_group_mask_document` | Add Reveal All to populated and empty groups; inspect thumbnail; paint and fill their masks. | Document-sized white mask, nonempty thumbnail, coverage edits succeed. | `tst_canvas_mask_domain::creationVariantsKeepDefaults` |
| `cmd_empty_layer` | Create an empty pixel layer; add Reveal All and From Transparency in separate fixtures; paint at a canvas edge. | Initial canvas is white; edits work without growing layer pixels. | `pictura-render::mask_creation_preserves_empty_and_group_defaults`, `tst_canvas_mask_domain::creationVariantsKeepDefaults` |
| `cmd_alpha_creation` | Add From Transparency to offset layers with partial alpha and no alpha. | Exact alpha or 255 inside source coordinates, zero elsewhere/default. | `pictura-render::mask_creation_preserves_alpha_coordinates` |
| `cmd_selection_creation` | Add both selection variants from an offset small coverage plane with partial samples and nonzero default. | Canvas samples equal selection or inverse at document coordinates; exterior is zero. | `pictura-render::mask_creation_preserves_selection_coordinates` |
| `cmd_creation_offcanvas` | Create each applicable mask kind from sources crossing negative and far canvas edges. | Every source sample is retained off-canvas and canvas remains editable. | `pictura-render::mask_creation_retains_offcanvas_samples` |
| Activating and clearing the target | Target a valid mask, clear, and target an invalid path. | Target reads correctly; invalid path refuses; no document/history mutation. | `tst_canvas_mask_domain::maskTargetLifecycle` |
| A mask round-trips through the adapter | Build a canvas view of an offset step mask; edit a canvas ROI and write it back. | Correct document-space edit, unchanged layer pixels and samples outside ROI. | `pictura-render::canvas_mask_adapter_preserves_document_coordinates` |
| A deleted mask empties the target | Activate then delete a raster mask. | Target reads empty. | `tst_canvas_mask_domain::maskTargetLifecycle` |
| `lm_group_mask_edit` | Repeat adapter write-back on a group and sample the resulting composite. | Group coverage and visible result follow the edit. | `pictura-render::canvas_mask_adapter_preserves_document_coordinates` |
| `cmd_empty_stored_rect` | Construct a valid data-less empty-rect mask with gray stored default on a nonempty document; edit its far corner. | Editing view starts at 255, corner updates, unrelated samples stay 255; building the view does not rewrite the gray source default or missing-data state. | `pictura-render::canvas_mask_adapter_preserves_document_coordinates` |
| Painting white and black on the mask | Paint white on hidden coverage and black on revealed coverage. | Coverage and composite follow; one Brush state per accepted stroke. | `tst_canvas_mask_domain::canvasMaskBasicEdits` |
| Filling the mask | Fill a selected mask region with black. | Correct coverage and one Fill state; pixels unchanged. | `tst_canvas_mask_domain::canvasMaskBasicEdits` |
| Filtering the mask | Commit a filter, then preview and cancel another filter on a shifted mask. | One Filter state on commit; cancelled preview restores exact samples and bounds. | `tst_canvas_mask_domain::importedMaskFilters`, `tst_canvas_mask_domain::canvasMaskCancelAndNoop` |
| No target keeps layer editing | Clear the mask target and paint the owning layer. | Layer pixels change; mask does not. | `tst_canvas_mask_domain::layerEditingWithoutMaskTarget` |

## Canvas domain, movement, display, and persistence

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cmd_canvas_paint` | Generate and open a small imported mask; use Brush, Pencil, and Eraser beyond both mask/layer rects. | Coverage changes at the requested points, original pixels and other mask samples do not. | `tst_canvas_mask_domain::importedMaskCanvasTools` |
| `cmd_canvas_fill` | Exercise each named fill family on a selected margin beyond an imported mask; keep a nonzero off-canvas sentinel. | Correct selected mask writes, no layer writes, sentinel retained. | `tst_canvas_mask_domain::importedMaskFillTools` |
| `cmd_canvas_filter` | Apply Invert and a destructive filter through a selected canvas margin beyond the imported rect. | Requested canvas coverage changes; out-of-selection and off-canvas sentinels remain exact. | `tst_canvas_mask_domain::importedMaskFilters` |
| `cmd_import_defaults` | Normalize first edits of offset imported masks with decoded explicit gray samples, exterior 255 and 128, and non-default density/feather. | Other raw samples and metadata survive; rendered unedited canvas coverage including feathered canvas edges is preserved. | `pictura-render::canvas_mask_preserves_import_defaults` |
| `cmd_data_less_import` | Generate/import a nonempty mask header without decoded coverage and with defaults 0 and 128; sample composite and solo, commit one local edit, then undo/redo. | Before edit both views show 255; outside the edit and its normal refinement reach coverage stays 255; undo restores no decoded data, original signed bounds/default/metadata; redo restores the edit. | `pictura-render::canvas_mask_data_less_import_preserves_visible_coverage`, `tst_canvas_mask_domain::dataLessImportFirstEditPreservesViews` |
| `cmd_read_only` | Open decoded and data-less generated masks; select each, show solo, and request Layers/Channels thumbnails. | Same mask data/rect/default/native planes, history index, and modified state; missing data remains missing and displays as 255. | `tst_canvas_mask_domain::importedMaskReadOnlyViews`, `tst_canvas_mask_domain::dataLessImportFirstEditPreservesViews` |
| `cmd_move_black` | Import a white-default mask, set both swatches white, and perform a whole-mask drag; inspect live preview and committed samples. | Exposed stored coverage is zero during preview and after release; moved values preserved. | `pictura-render::canvas_mask_moves_keep_black_exterior`, `tst_canvas_mask_domain::wholeMaskMoveBlackPreview` |
| `cmd_move_return` | Parameterize integer offsets through all four edges and corners, partial and fully outside, with values 0, 1, 64, 128, 254, 255; reverse offsets. | Exact original samples return; no thresholding or clipping. | `pictura-render::canvas_mask_moves_restore_every_edge` |
| `cmd_offcanvas_edit` | Move coverage outside; separately paint, fill, and filter an on-canvas ROI; bring retained coverage back. | Retained off-canvas values remain byte-identical. | `pictura-render::canvas_mask_edit_preserves_offcanvas_samples`, `tst_canvas_mask_domain::importedMaskFilters` |
| `cmd_native_move` | Repeat out/back moves and first-domain expansion for native u16 and f32 masks, including minimum positive samples whose 8-bit proxies are zero. | Native samples are exact; float bit patterns survive; untouched native off-canvas samples survive canvas edits. | `pictura-render::canvas_mask_native_moves_preserve_precision` |
| `cmd_move_target` | Drag an unlinked targeted mask, then a layer carrying a linked mask. | First leaves layer bounds/pixels fixed; second carries mask with layer; both expose black and have one move state. | `tst_canvas_mask_domain::maskMoveRouting` |
| `cmd_thumb_black` | Show a new small square mask in a wide canvas under both thumbnail contents options; inspect image alpha and grayscale. | Document aspect, opaque black margins, correctly positioned white content; color-layer thumbnail options remain functional. | `tst_layers_snapshot::maskThumbnailUsesOpaqueCanvasCoverage` |
| `cmd_thumb_bounded` | Take small thumbnails of large decoded and data-less enabled/disabled masks under a sample counter and no-materialization guard. | Identical coverage thumbnails, 255 for missing data without source mutation; only output-grid samples touched. | `pictura-app::canvas_mask_thumbnail_samples_only_grid` |
| `cmd_refusal` | Pin a low governor budget for creation/edit/move workspace and Properties Invert on decoded and data-less masks; test overflowing coordinates and selection-move destinations. | Exact original mask/native data including missing-data state/default, pixels, target, selection, modified state, and history; visible failure at the application layer. | `pictura-render::canvas_mask_refusals_are_atomic`, `tst_canvas_mask_domain::canvasMaskRefusalIsAtomic` |
| `cmd_undo` | Commit the first paint/fill/filter/move that expands an imported mask; undo then redo. | One normal state; undo restores original geometry/data/default; redo restores expanded data and off-canvas sentinels. | `tst_canvas_mask_domain::canvasMaskEditUndoRedo` |
| `cmd_cancel` | Cancel first stroke/filter preview/whole-mask drag on decoded and data-less imported masks; separately commit zero-offset whole and selected moves. | Exact original geometry/data/default/metadata and history with no eager normalization; missing-data imports remain data-less. | `tst_canvas_mask_domain::canvasMaskCancelAndNoop` |
| `cmd_saved_return` | At depths 8, 16, 32 move a mask fully outside, write PSD and PSB to temporary files, reopen, move back; include signed bounds and non-default properties. | Exact saved-depth samples return; zero exterior and properties survive; opening does not rewrite storage. | `pictura-codec::offcanvas_mask_round_trips_at_native_depth` |

## Selected mask coverage

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mask_content_move` | Drag selected coverage with a mask active, including a destination past its old rect. | Coverage moves, pixels do not, attachment stays, bounds grow only as needed. | `tst_canvas_mask_domain::selectedMaskMoveRoutesCoverage` |
| `mask_content_move_background` | Set background white; drag a fully selected white patch. | Source becomes zero and hides its layer, independently of swatch. | `tst_canvas_mask_domain::selectedMaskMoveBlackBackground` |
| `mask_content_move_duplicate` | Alt-drag a mask patch partly beyond the canvas. | Original and copied coverage retained; no new layer, active-layer switch, or pixel-clone preview. | `tst_canvas_mask_domain::selectedMaskAltRetainsCoverage` |
| `mask_content_move_ctrl` | Ctrl-drag inside a selection with a selection tool and mask active. | Coverage moves; no new selection gesture starts. | `tst_canvas_mask_domain::selectedMaskCtrlRoutesCoverage` |
| `mask_content_move_history` | Commit selected mask movement, undo and redo; include complete exit beyond canvas. | Exactly one Move Selection state; ordinary visible selection clipping; retained mask data survives. | `tst_canvas_mask_domain::selectedMaskMoveHistory` |
| `mask_content_move_noop` | Press/release at identical coordinates on an imported mask selection. | Samples, bounds, and history unchanged. | `tst_canvas_mask_domain::canvasMaskCancelAndNoop` |
| `cmd_selected_offcanvas` | Move selected gray/white coverage fully beyond each edge; translate the whole mask to recover the destination. | Destination values retained exactly; cleared source remains zero. | `pictura-render::selected_mask_move_preserves_offcanvas_samples` |
| `cmd_selected_partial` | Move coverage 200 under selection 128 onto a disjoint zero destination with white background. | Rounded source and destination coverage both equal 100. | `pictura-render::selected_mask_move_blends_toward_black` |

## Properties refinement and thumbnail contract regressions

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Density change undoes | Change mask density from 255 to 128 through Properties, commit, and undo. | One state; undo restores density 255 without changing coverage samples. | `tst_canvas_mask_domain::propertiesMaskRefinementUndo` |
| Invert flips the mask permanently | Invert a mask with in-canvas value 40, exterior 0, and an off-canvas gray sentinel; undo. | Canvas value becomes 215, default stays 0, sentinel is unchanged, and undo restores the original mask. | `pictura-render::canvas_mask_properties_invert_preserves_retained_samples` |
| `cmd_properties_invert_retained` | Keep partial/native coverage outside the canvas and a smaller active pixel selection; click the actual Properties Invert button, then undo/redo. | Whole canvas is inverted despite selection, off-canvas/native samples remain exact, exterior is black, layer pixels/selection stay unchanged, native output reflects the edit, and one Invert state restores exact storage on undo/redo. | `pictura-render::canvas_mask_properties_invert_preserves_retained_samples`, `tst_canvas_mask_domain::propertiesInvertPreservesOffcanvasCoverage` |
| `lpr_thumb_aspect` | Show regular layers on wide and tall documents. | Drawn color thumbnail keeps document aspect with unused box area empty. | `tst_layers_snapshot::canvasAspectThumbnailRegression` |
| `lpr_thumb_source_aspect` | Show a full-width layer in a tall document under Entire Document. | Output carries document aspect; rightmost column is content without a transparent strip. | `tst_layers_snapshot::canvasAspectThumbnailRegression` |
| `lpr_thumb_square_card` | Show type, adjustment, fill, and synthetic rows in a nonsquare document alongside a pixel row; inspect card bounds and image fitting. | Icon/fill/synthetic cards remain 1:1, pixel card remains document-letterboxed; fitting a document-aspect raster mask does not change those cards or vector behavior. | `tst_layers_snapshot::canvasAspectThumbnailRegression` |
| `lpr_thumb_sub_canvas_layer` | Show a small offset raster layer in a nonsquare document under Entire Document. | Color image preserves document aspect and position with transparent checkerboard margins; raster-mask margins follow the separate opaque rule. | `tst_layers_snapshot::canvasAspectThumbnailRegression` |
| `lpr_thumb_mask_aspect` | Display a newly created mask on a small layer, then an off-canvas mask, under both thumbnail contents options; compare source storage before/after reads. | Raster-mask image is opaque document-aspect grayscale in both modes; zero margins are black and off-canvas clipping affects output only; existing card layout and layer-color thumbnail rules remain unchanged. | `tst_layers_snapshot::maskThumbnailUsesOpaqueCanvasCoverage` |

## Execution and gate status

No application or Rust build/test is run while proposing. During apply, write
the named tests before implementation; finish all code changes before the
first compile, using at most 80 percent of available CPUs. Then run the focused
engine/codec suites and Qt suites with the repository's reporting conventions,
followed by the relevant gates. `scripts/check-test-plan.py` will report the
not-yet-written references until those tests land; do not mark them manual or
weaken the checker to make a planning-only change appear implemented.
