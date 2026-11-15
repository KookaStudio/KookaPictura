# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`. Oracle tests self-skip when `pdfimages`,
`pdftoppm`, or `psd-tools` is absent; CI installs them.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pdf_zip` | Write with ZIP; inflate the image stream | `FlateDecode`; exact composite samples | `pictura-codec::pdf_zip_decodes_to_composite` |
| `pdf_jpeg` | Save with JPEG Maximum and Low through the app; render | `DCTDecode`; close to composite; Low smaller | `tst_save_pdf::jpegCompressionRenders` |
| `pdf_no_compression` | Write with None | No `/Filter`; length = w × h × n | `pictura-codec::pdf_no_compression_writes_raw` |
| `pdf_jpeg2000` | Write an 8-bit RGB document at Acrobat 6 with JPEG 2000 at Maximum; `pdfimages -list`, `pdftoppm` | `JPXDecode`; lister reports jpx; render close to the composite | `pictura-codec::pdf_jpeg2000_writes_codestream`, `pictura-codec::pdf_jpeg2000_poppler_lists_jpx`, `tst_save_pdf::jpeg2000IsEnabled` |
| `pdf_jpeg2000_quality` | Same document at Low and Maximum | Both render close; Low file smaller | `pictura-codec::pdf_jpeg2000_quality_changes_size` |
| `pdf_jpeg2000_gated` | Select JPEG 2000 at Acrobat 5, with a 16-bit document and Convert 16 Bit/Channel off, and under PDF/A-1b | Image uses `FlateDecode` | `pictura-codec::pdf_jpeg2000_gated_to_zip` |
| `pdf_jpeg2000_disabled` | Show the Compression page at Acrobat 5, for a 16-bit document with Convert 16 Bit/Channel Image To 8 Bit/Channel off, under PDF/A-1b, and for Bitmap/Indexed | JPEG 2000 listed but not selectable | `tst_save_pdf::jpeg2000IsDisabled` |
| `pdf_embeds_psd` | Two-layer doc, Preserve Editing on | PieceInfo PSD stream decodes to two layers | `pictura-codec::pdf_embeds_psd_payload` |
| `pdf_reopen_layers` | Save, then `File > Open` the PDF | Same layers and composite | `tst_save_pdf::reopenRestoresLayers` |
| `pdf_layers_off` | Preserve on, Layers off | Embedded PSD has no layer tree | `pictura-codec::pdf_layers_off_embeds_flat_psd` |
| `pdf_spot_colors` | Document with one spot and one alpha extra channel; Preserve on, both options on | Embedded PSD carries both channels and both names | `pictura-codec::pdf_spot_payload_keeps_spot_and_alpha` |
| `pdf_spot_colors_off` | Same document, Spot Colors off | Embedded PSD drops the spot, keeps the alpha and its name | `pictura-codec::pdf_spot_payload_drops_spot_only` |
| `pdf_alpha_off_keeps_spot` | Same document, Alpha Channels off | Embedded PSD drops the alpha, keeps the spot under its name | `pictura-codec::pdf_spot_payload_drops_alpha_only` |
| `pdf_flat_warns` | Layered doc, Preserve off, confirm | Features warning before writing | `tst_save_pdf::preserveOffWarnsForLayers` |
| `pdf_open_foreign_refused` | Open a PDF without the payload | Fails; document unchanged | `pictura-codec::pdf_read_refuses_foreign_pdf` |
| `pdf_dialog_pages` | Open the dialog | Five pages with their controls | `tst_save_pdf::dialogHasFivePages` |
| `pdf_save_options` | Open with a SaveContext carrying a spot document flag, then without one; open it with a Working Gray proof name | Spot Colors enabled then disabled; Use Proof Setup names the active setup | `tst_save_pdf::spotColorsFollowTheDocument`, `tst_save_pdf::proofSetupLabelFollowsTheSetup` |
| `pdf_compat_gates` | Set Acrobat 4 | 16→8 checked and disabled; Security names RC4 40-bit | `tst_save_pdf::compatibilityGatesControls` |
| `pdf_summary` | Summary for PDF/X-1a | Lists settings; transparency warning | `tst_save_pdf::summaryListsSettings` |
| `pdf_use_proof_setup` | sRGB doc; Use Proof Setup on (Working CMYK), No Conversion, Include Destination; `pdftoppm` | ICCBased N 4 with the bundled CMYK bytes; render close | `pictura-codec::pdf_use_proof_setup_converts_to_device`, `pictura-codec::pdf_use_proof_setup_renders_like_composite` |
| `pdf_use_proof_override` | Convert To Destination sRGB with Use Proof Setup on | Output follows the destination; control disabled; Summary warns | `pictura-codec::pdf_output_conversion_overrides_proof`, `tst_save_pdf::proofSetupLocksUnderOutputConversion` |
| `pdf_use_proof_off` | Use Proof Setup off, No Conversion, ICC Profile on | Image tagged with the document profile | `pictura-codec::pdf_use_proof_off_keeps_document_tag` |
| `pdf_use_proof_16bit` | Write a 16-bit document with Use Proof Setup on | Convert 16 Bit/Channel forced on; image samples are 8-bit | `pictura-codec::pdf_use_proof_setup_writes_eight_bit` |
| `pdf_use_proof_no_profile` | Use Proof Setup on (Working CMYK) with Profile Inclusion Policy Don't Include Profiles | Image still tagged with the proof ICC profile | `pictura-codec::pdf_use_proof_setup_ignores_profile_policy` |
| `pdf_use_proof_live_text` | Type layer written as live text with Use Proof Setup on | Text fill follows the proof conversion | `pictura-render::pdf_live_text_colour_follows_proof` |
| `proof_defaults` | Open the app with no stored proof setup | Working CMYK, relative, BPC on, simulations off | `tst_proof_setup::defaultsAreWorkingCmyk` |
| `proof_persist` | Select Working Gray, turn Proof Colors on, restart | Working Gray and Proof Colors survive | `tst_proof_setup::setupSurvivesRestart` |
| `proof_preview` | Preview one colour through Working CMYK and without proofing | Colours differ; the proofed value matches a directly built lcms2 proofing transform | `pictura-color::proof_preview_changes_colors` |
| `proof_simulations` | Preview with paper simulation alone, black-ink alone, both off | Each differs from the others | `pictura-color::proof_simulations_differ` |
| `proof_alpha` | Preview a half-transparent RGBA8 pixel | Alpha unchanged; colour changes | `pictura-color::proof_alpha_untouched` |
| `proof_menu` | Choose View > Proof Setup > Working Gray | Working Gray active and checked; others not | `tst_proof_setup::menuSelectsSetup` |
| `proof_colors_toggle` | Press `Ctrl+Y` twice | Checked and proof on, then unchecked and proof off | `tst_proof_setup::proofColorsToggles` |
| `proof_title` | Proof Colors on with Working Gray | Title contains the Working Gray name | `tst_proof_setup::titleNamesProofSetup` |
| `proof_canvas` | Toggle Proof Colors on and off | Canvas pixels change to the proof preview and back | `tst_proof_setup::canvasShowsProof` |
| `proof_view_only` | Toggle Proof Colors on and off | Document, composite, history, and modified state unchanged | `tst_proof_setup::proofingIsViewOnly` |
| `proof_mask_views` | Enter Quick Mask, then a solo-mask view, with Proof Colors on | Mask views render unproofed pixels | `tst_proof_setup::proofSkipsMaskViews` |
| `proof_custom_dialog` | Accept with Simulate Black Ink on; reopen, change device, cancel | OK applies the simulation; Cancel keeps every value | `tst_proof_setup::customDialogAppliesAndCancelKeeps` |
| Spot payload cross-check | Write a PDF with a spot channel; extract the embedded PSD and read it with psd-tools | Channel count and names match the writer | `pictura-codec::pdf_spot_payload_matches_psd_tools` |
| Real proofing against a physical proof | Compare a proofed screen to a printed proof of the same document | Not claimed: the engine's reference is lcms2, not a measured press | manual: needs a calibrated display and a physical proof; lcms2 parity is automated above |
