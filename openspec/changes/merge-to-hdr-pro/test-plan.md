# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Reject two sources | Select two valid matching files in the picker. | Proceed is disabled or refused with the minimum-three explanation. | tst_merge_to_hdr::rejectsTwoSources |
| Reject a high-depth source | Select a 16-bit PNG with two valid files. | A visible error reports the precision limit and no document opens. | tst_merge_to_hdr::rejectsDeepSources |
| Complete the missing exposure values | Select three matching files with missing metadata; try blank and non-finite EV100, enter explicit values, then cancel. | Missing rows start empty and block progress until finite values are entered; inputs remain unchanged. | tst_merge_to_hdr::requiresExplicitExposure |
| Add clean saved open documents | Offer clean saved, dirty, and unsaved documents. | Only the clean saved document is eligible; no document or file changes. | tst_merge_to_hdr::addsCleanSavedDocuments |
| Recover scene radiance | Merge an analytical three-exposure bracket including a clipped highlight and compare each result to median-normalized scene radiance. | Values are monotonic and within 0.015 absolute error after 8-bit source quantization, including a highlight above white. | pictura-core::recovers_scene_radiance |
| Reuse a response curve | Replay a saved curve on the same bracket; try malformed and unsupported curves. | Samples match exactly; invalid curves fail without publication. | pictura-core::replays_saved_curve, tst_merge_to_hdr::rejectsInvalidCurve |
| Recover a translated exposure | Align a textured bracket with a known in-range offset. | The translation is recovered and out-of-frame samples are excluded. | pictura-core::recovers_translation |
| Reject failed alignment | Align textureless, ambiguous periodic/smooth, and out-of-range shifted brackets. | Both fail deterministically without publication. | pictura-core::rejects_failed_alignment, pictura-core::rejects_ambiguous_periodic_and_smooth_registration, tst_merge_to_hdr::alignmentFailureIsAtomic |
| Resolve conflicting moving pixels | Choose each of two bases for a moving patch. | The selected thumbnail is indicated and its usable pixel is used. | pictura-core::selects_base_on_disagreement, tst_merge_to_hdr::selectsGhostBase |
| Fall back for saturated and transparent samples | Merge highlight-clipped, shadow-clipped, mixed zero-weight, and all-transparent pixels, including ties. | Fallback source, alpha and transparent black match the design deterministically. | pictura-core::saturation_and_alpha_fallback, pictura-core::fuses_unclipped_channels_in_saturated_colors |
| Adjust preview brightness | Change preview brightness and compare display and floating-point samples. | Display changes; samples remain byte-identical. | tst_merge_to_hdr::previewBrightness |
| Publish or cancel the merge | Use a modal harness to accept 32-bit output and separately cancel at picker, preview, and conversion. | One correct untitled float document opens on acceptance; cancellation leaves all inputs and document count unchanged. | tst_merge_to_hdr::publishesOrCancels, pictura-app::merged_native_samples_survive_psd_and_psb |
| Convert the merged output | Use the modal harness to accept 8- and 16-bit conversions with known parameters. | Each opens one document of the chosen depth with expected converted pixels. | tst_merge_to_hdr::convertsIntegerOutput, pictura-app::converted_sixteen_bit_samples_keep_precision |
| Reject an over-budget job | Inject reservation refusal and checked-estimate overflow. | Visible failure, released resources, unchanged inputs and document count. | tst_merge_to_hdr::budgetFailureIsAtomic, pictura-core::rejects_memory_overflow |

Deferred oracle: compare a three-exposure result against Debevec within the contract's tolerance when a reference oracle and agreed tolerance are available. This is not claimed as verified parity. Coverage: manual: no reference oracle or agreed tolerance is supplied.
