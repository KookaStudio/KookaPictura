# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`. Oracle tests self-skip when `pdftoppm`, `gs`, or
`qpdf` is absent; CI installs all three.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pdf_save_as_format` | Read the Save As filters; map the PDF filter to a suffix | `Photoshop PDF (*.PDF *.PDP)` listed; suffix `.pdf` | `tst_save_pdf::saveAsListsPhotoshopPdf` |
| `pdf_path_becomes_pdf` | Edit a document, save it through the PDF edge to `poster.pdf` | File is a PDF; path is `poster.pdf`; not modified | `tst_save_pdf::pdfSaveRecordsPath` |
| `pdf_resave_shows_dialog` | Save with JPEG, edit, Save again | Dialog opens with JPEG selected | `tst_save_pdf::resaveReopensDialogWithSettings` |
| `pdf_cancel_writes_nothing` | Open the dialog from Save As, cancel | No file; path and modified unchanged | `tst_save_pdf::cancelWritesNothing` |
| `pdf_atomic_failure` | Save to a path in a missing directory | Failure; no file | `tst_save_pdf::failedWriteCreatesNoFile` |
| `pdf_page_size` | Write 600 × 300 px at 300 ppi | MediaBox `[0 0 144 72]` | `pictura-codec::pdf_page_size_follows_resolution` |
| `pdf_renders_composite` | Write RGB with ZIP; render with `pdftoppm` at the document ppi | Pixels equal the composite | `pictura-codec::pdf_render_matches_composite` |
| `pdf_valid_structure` | Write each level; `qpdf --check` | No errors; header version matches | `pictura-codec::pdf_qpdf_check_passes` |
| `pdf_live_text` | Pixel layer + type layer `Kooka Pictura`; write with ZIP; `pdftotext`, `pdffonts` | `Type0`/`Identity-H`, `FontFile2`, `ToUnicode`; extracted text `Kooka Pictura`; image samples equal the composite without the type layer | `pictura-render::pdf_type_layer_is_live_text`, `pictura-render::pdf_live_text_extracts` |
| `pdf_live_text_renders` | Same file; `pdftoppm` at the document ppi | Mean channel error against the full composite under 3; solid ink renders red | `pictura-render::pdf_live_text_renders_like_composite` |
| `pdf_live_text_covered` | Add a pixel layer above the type layer | No font; image equals the full composite | `pictura-render::pdf_covered_type_stays_in_image` |
| `pdf_live_text_opacity` | Type layer at 50%; Acrobat 6, then Acrobat 4 | `/ca 0.5` ExtGState + font; then no font | `pictura-render::pdf_faded_type_follows_transparency` |
| `pdf_live_text_colour` | Red type; Convert to Photocraft Coated CMYK, Include Destination | Text filled with `/CS0 cs` and four components: red as magenta and yellow ink | `pictura-render::pdf_live_text_colour_follows_conversion` |
| `pdf_live_text_app` | Add a type layer in the app; `pdf_save` | A `Type0` font with an embedded font file; `pdftotext` reads the typed string | `tst_save_pdf::typeLayerIsLiveText` |
| Edit the text in a PDF editor | Open a saved PDF in Acrobat / OnlyOffice / LibreOffice Draw; edit the type | The words are editable text in the embedded font | manual: needs an interactive PDF editor; text extraction and font embedding are automated above |
| `pdf_gray` | Write a Grayscale document | One component; renders as the gray composite | `pictura-codec::pdf_gray_writes_one_component` |
| `pdf_cmyk` | Write a CMYK-source document, No Conversion; render | DeviceCMYK; renders as the composite | `pictura-codec::pdf_cmyk_source_writes_device_cmyk` |
| `pdf_lab` | Write a Lab-source document | `/Lab` with D50 white point | `pictura-codec::pdf_lab_source_writes_lab` |
| `pdf_indexed` | Write an unedited Indexed document; render | `/Indexed` 256-entry palette; renders as the composite | `pictura-codec::pdf_indexed_source_writes_palette` |
| `pdf_bitmap_ccitt` | Write an unedited Bitmap document; render | 1 bpc, `CCITTFaxDecode` `K -1`; same black/white pixels | `pictura-codec::pdf_bitmap_uses_ccitt_g4` |
| `pdf_multichannel_refused` | Write a Multichannel document | Error naming Multichannel; no bytes | `pictura-codec::pdf_multichannel_is_refused` |
| `pdf_icc_embed` | Write Adobe RGB with ICC Profile on, then off | ICCBased with the profile bytes; then DeviceRGB | `pictura-codec::pdf_icc_profile_embedding_follows_option` |
| `pdf_alpha_smask` | Transparent pixel at Acrobat 6 | `SMask` sample 0 at that pixel | `pictura-codec::pdf_alpha_becomes_soft_mask` |
| `pdf_alpha_flatten` | Same at Acrobat 4 | No `SMask`; pixel white | `pictura-codec::pdf_acrobat4_flattens_alpha` |
| `pdf_zip` | Write with ZIP; inflate the image stream | `FlateDecode`; exact composite samples | `pictura-codec::pdf_zip_decodes_to_composite` |
| `pdf_jpeg` | Save with JPEG Maximum and Low through the app; render | `DCTDecode`; close to composite; Low smaller | `tst_save_pdf::jpegCompressionRenders` |
| `pdf_no_compression` | Write with None | No `/Filter`; length = w × h × n | `pictura-codec::pdf_no_compression_writes_raw` |
| `pdf_jpeg2000_disabled` | Open the Compression page | JPEG2000 listed, item disabled | `tst_save_pdf::jpeg2000IsDisabled` |
| `pdf_downsample` | 300 ppi, Bicubic 150 above 225 | Image half size; MediaBox unchanged | `pictura-codec::pdf_downsample_above_threshold` |
| `pdf_downsample_threshold` | 200 ppi, same settings | Image keeps its size | `pictura-codec::pdf_downsample_below_threshold_keeps_size` |
| `pdf_downsample_methods` | Checker ÷ 2 with each method | Mean / source sample / weighted | `pictura-codec::downsample_methods_differ` |
| `pdf_16bit` | Unedited 16-bit RGB at Acrobat 6, conversion off | 16 bpc; source samples | `pictura-codec::pdf_sixteen_bit_kept` |
| `pdf_16bit_forced` | Same at Acrobat 5 | 8 bpc; option locked | `pictura-codec::pdf_sixteen_bit_forced_below_15` |
| `pdf_embeds_psd` | Two-layer doc, Preserve Editing on | PieceInfo PSD stream decodes to two layers | `pictura-codec::pdf_embeds_psd_payload` |
| `pdf_reopen_layers` | Save, then `File > Open` the PDF | Same layers and composite | `tst_save_pdf::reopenRestoresLayers` |
| `pdf_layers_off` | Preserve on, Layers off | Embedded PSD has no layer tree | `pictura-codec::pdf_layers_off_embeds_flat_psd` |
| `pdf_flat_warns` | Layered doc, Preserve off, confirm | Features warning before writing | `tst_save_pdf::preserveOffWarnsForLayers` |
| `pdf_open_foreign_refused` | Open a PDF without the payload | Fails; document unchanged | `pictura-codec::pdf_read_refuses_foreign_pdf` |
| `pdf_notes` | Note "check the sky", Notes on / off | `/Text` annotation with contents / none | `pictura-codec::pdf_notes_become_text_annotations` |
| `pdf_thumbnail` | 400 × 200 with thumbnails | `/Thumb` 106 × 53 | `pictura-codec::pdf_thumbnail_is_106px` |
| `pdf_linearized` | Fast Web View with and without security; `qpdf --check-linearization` | Linearized, no errors | `pictura-codec::pdf_fast_web_view_linearizes` |
| `pdf_view_after` | Save with View PDF After Saving | Viewer hook receives the saved path | `tst_save_pdf::viewAfterSavingOpensFile` |
| `pdf_convert_cmyk` | RGB → Photocraft Coated CMYK, Include Destination; render | ICCBased N 4 with the bundled bytes; close render | `pictura-codec::pdf_convert_to_cmyk_tags_destination` |
| `pdf_profile_inclusion` | Same, Don't Include Profiles | DeviceCMYK | `pictura-codec::pdf_dont_include_profiles_writes_device` |
| `pdf_x1a` | RGB as PDF/X-1a:2001 | 1.3; DeviceCMYK; GTS_PDFX intent; info keys | `pictura-codec::pdf_x1a_conformance` |
| `pdf_x3` | RGB as PDF/X-3:2002 | 1.3; ICCBased RGB; X-3 version key | `pictura-codec::pdf_x3_conformance` |
| `pdf_x4` | Transparent doc as PDF/X-4:2008 | 1.6; SMask kept; XMP `pdfxid` | `pictura-codec::pdf_x4_conformance` |
| `pdf_a1b` | PDF/A-1b:2005 | 1.4; no SMask/encryption; `pdfaid`; GTS_PDFA1 | `pictura-codec::pdf_a1b_conformance` |
| PDF/X and PDF/A validation | Run Acrobat Preflight or veraPDF on the four standard files | No conformance errors | manual: no PDF/X or PDF/A validator is installable in CI (Acrobat is proprietary; veraPDF is a Java distribution); the structural rules are automated above |
| `pdf_standard_locks` | Select PDF/X-1a in the dialog | Compatibility Acrobat 4, disabled; Security disabled | `tst_save_pdf::standardLocksSettings` |
| `pdf_open_password` | Each level with open password `kooka`; `qpdf` | Refused without, opens with; level's cipher | `pictura-codec::pdf_open_password_each_level` |
| `pdf_permissions` | Permissions password, no printing, no copying; `qpdf --show-encryption` | Opens; print and extract not allowed | `pictura-codec::pdf_permissions_restrict` |
| `pdf_security_not_in_presets` | Preset text of options with passwords | No password or permission keys | `pictura-codec::joboptions_preset_drops_security` |
| `pdf_builtin_presets` | Read the preset combo; pick Smallest File Size | Twelve bracketed built-ins first; JPEG Low, Bicubic 100 | `tst_save_pdf::builtInPresetsListed` |
| `pdf_preset_modified` | Choose a preset, change a setting | Name gains ` (Modified)` | `tst_save_pdf::editMarksPresetModified` |
| `pdf_save_preset` | Save Preset as `Web proof` | File written and listed | `tst_save_pdf::savePresetWritesFile` |
| `pdf_joboptions_roundtrip` | Options → joboptions → options | Equal preset settings | `pictura-codec::joboptions_round_trip` |
| `pdf_joboptions_acrobat` | Parse an Acrobat-style file | Known keys and description read; unknown ignored | `pictura-codec::joboptions_parses_acrobat_file` |
| `pdf_presets_manager` | New, Edit, Delete a user preset; try built-ins | File written, rewritten, removed; built-ins read-only | `tst_save_pdf::presetManagerEditsUserPresets` |
| `pdf_dialog_pages` | Open the dialog | Five pages with their controls | `tst_save_pdf::dialogHasFivePages` |
| `pdf_compat_gates` | Set Acrobat 4 | 16→8 checked and disabled; Security names RC4 40-bit | `tst_save_pdf::compatibilityGatesControls` |
| `pdf_summary` | Summary for PDF/X-1a | Lists settings; transparency warning | `tst_save_pdf::summaryListsSettings` |
| Layered reopen in Photoshop | Open a Pictura Photoshop PDF in Adobe Photoshop | Not claimed: the payload is Pictura's own | manual: Photoshop is proprietary and its layered-PDF layout unpublished (design D6); recorded as an approximation, not a check |
| Real Save As / viewer on desktop | Save As → Photoshop PDF on a desktop session; View PDF After Saving | Native picker, dialog, and the system viewer open | manual: offscreen QPA shows no native picker or external viewer; the dialog, routing, and viewer hook are automated above |
