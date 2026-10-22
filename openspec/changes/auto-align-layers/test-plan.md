# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Reposition Only aligns a known translation | Build two layers with the same content offset by `(dx, dy)` and run Reposition Only | The non-reference layer moves by `(dx, dy)` within one pixel; no rotation or scale is introduced; the reference is unchanged | `pictura-render::reposition_only_recovers_translation` |
| A locked layer is the reference | Select a locked layer and a movable layer and run Auto-Align | The locked layer is byte-identical and the movable layer aligns to it | `pictura-render::locked_layer_is_the_reference` |
| The composition centre picks the reference without a lock | Spread two eligible layers across the canvas with no lock | The layer nearest the document centre is the reference | `pictura-render::composition_centre_picks_the_reference` |
| Perspective aligns a synthetic quadrilateral | Warp a synthetic layer by a known homography and run Perspective | The transformed corners match within two pixels | `pictura-render::perspective_recovers_a_synthetic_quad` |
| Scene Collage preserves shape | Rotate and translate a rectangle and run Scene Collage | The applied map is a similarity with no shear or perspective; the rectangle stays a rectangle | `pictura-render::scene_collage_applies_a_similarity` |
| Auto is a reduced supported-mode heuristic | Run Auto on a pair that Reposition Only cannot align and Perspective can | Auto's residual is no worse than Reposition Only's and uses a supported mode, not CS6's Perspective/Cylindrical choice | `pictura-render::auto_picks_the_better_supported_mode` |
| Unsupported modes are refused | Request Cylindrical, Spherical, and Lens Correction | Each returns an unsupported error and no layer changes | `pictura-render::unsupported_modes_are_refused`, `tst_auto_align::unsupportedModesDisabled` |
| Ineligible layers are skipped | Select one eligible raster layer plus groups/adjustment/fill/vector/type/Smart Object/Background layers | The command is disabled and no registration runs | `pictura-render::ineligible_layers_are_skipped`, `tst_auto_align::menuDisabledWithoutTwoEligibleLayers`, `tst_auto_align::ineligibleKindsSkipped` |
| Cancel records nothing | Start a registration and cancel it before commit | No transform changes and no history state is added | `tst_auto_align::cancelRecordsNothing` |
| A mid-apply refusal leaves the stack unchanged | Force one computed placement to be refused at apply time | No layer changes are committed and no history state is added | `pictura-render::mid_apply_refusal_leaves_document_unchanged` |
| Repeated runs are identical | Run the same Auto-Align twice on unchanged layers | Layer pixels and transforms are identical both times | `pictura-render::auto_align_is_deterministic` |
| One history state and undo | Complete Auto-Align and undo | Exactly one `Auto-Align Layers` state exists and undo restores every affected layer | `tst_auto_align::oneHistoryStateAndUndo` |
