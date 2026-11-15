# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raster-determinism` | Encode the same fixture and options twice | Both byte sequences are identical | `pictura-codec::save_is_deterministic` |
| `raster-atomic-failure` | Encode to an unwritable target | Failure reported and no temp or partial file remains | `pictura-codec::save_atomic_failure` |
| `raster-atomic-set` | Fail one member of a multi-file set during encode | No member of the set exists at its final name | `pictura-codec::save_atomic_set` |
| `raster-atomic-set-rename` | Fail one member of a multi-file set during the rename phase | Already-renamed members are removed and no member exists at its final name | `pictura-codec::save_atomic_set_rename` |
| `bmp-depth-header` | Write 1/4/8/16/24/32-bit BMPs and parse the header | Header depth matches and the file decodes at size | `pictura-codec::bmp_depth_headers` |
| `bmp-rle` | Write 8-bit Windows BMP with RLE, and request RLE for 24-bit | RLE8 decodes identically; the invalid request is refused | `pictura-codec::bmp_rle_rules` |
| `bmp-flip` | Write Windows BMP with Flip Row Order on | Negative height and an unflipped decode | `pictura-codec::bmp_flip_rows` |
| `bmp-os2` | Write the OS/2 form at 1/4/8/24 bits | OS/2 header present and each image decodes | `pictura-codec::bmp_os2_header` |
| `bmp-os2-limits` | Request OS/2 with Flip, a 16-bit Advanced Mode or 32 bits | Each combination is refused without a file | `pictura-codec::bmp_os2_limits` |
| `bmp-advanced-16` | Write each 16-bit Windows layout | Pixels decode with the chosen channel layout | `pictura-codec::bmp_advanced_16_layouts` |
| `bmp-alpha-32` | Write a transparent pixel at 32 bits | Decoded alpha matches | `pictura-codec::bmp_32_alpha` |
| `png-alpha` | Write a transparent pixel as PNG | Decoded alpha matches | `pictura-codec::png_alpha_round_trip` |
| `png-interlace-adam7` | Write interlaced and non-interlaced | IHDR interlace is 1 and both decode the same | `pictura-codec::png_adam7_round_trip` |
| `png-compression-level` | Write at the lowest and highest levels | Sizes differ and pixels match | `pictura-codec::png_compression_levels` |
| `tiff-compression-tags` | Write None/RLE/LZW/ZIP/JPEG | Tag 259 is 1/32773/5/8/7 and each decodes | `pictura-codec::tiff_compression_tags` |
| `tiff-jpeg-limits` | Request TIFF JPEG for a transparent, non-8-bpc, CMYK or oversized image | Each unsupported request is refused without a file | `pictura-codec::tiff_jpeg_limits` |
| `tiff-planar` | Write Per Channel | Tag 284 is 2 and each channel has strips | `pictura-codec::tiff_planar_strips` |
| `tiff-byte-order` | Write a flat TIFF in Macintosh and IBM PC order | Macintosh is `MM`, IBM PC is `II`, and tags follow the header | `pictura-codec::tiff_byte_order_honors_option_for_flat_files` |
| `tiff-layered-byte-order` | Write a layered TIFF with the IBM PC option | Header is `MM` and tag 37724 is present so container and PSD payload agree | `pictura-codec::layered_tiff_is_big_endian` |
| `tiff-predictor` | Write LZW and ZIP with the internal predictor where it applies | Tag 317 declares predictor 2 when used and the file decodes | `pictura-codec::tiff_predictor_tag` |
| `tiff-alpha` | Write with Save Transparency on and off in RGB, grayscale, CMYK and Lab | Extra sample present, then the pixel is opaque and composited over the mode's white | `pictura-codec::tiff_alpha_flatten`, `pictura-codec::tiff_transparency_off_composites_white`, `pictura-codec::tiff_transparency_off_composites_cmyk_white`, `pictura-codec::tiff_transparency_off_composites_lab_white` |
| `tiff-metadata` | Write a document with all metadata blocks | Tags 34675/34665/33723/700/34377 present | `pictura-codec::tiff_metadata_tags` |
| `tiff-modes` | Write Gray/RGB/CMYK/Lab/Indexed | Tag 262 is 0-1/2/5/8/3, Indexed also carries ColorMap 320, and each decodes | `pictura-codec::tiff_mode_photometric` |
| `tiff-pyramid-subifds` | Write with Save Image Pyramid | Tag 330 lists a reduced-resolution SubIFD whose dimensions and strip decode, and IFD0 stays full size | `pictura-codec::tiff_pyramid_subifds` |
| `tiff-layered` | Write a layered document | Tag 37724 layer count and channel data match | `pictura-codec::tiff_layer_section_round_trip` |
| `tiff-layer-compression-bytes` | Write RLE and ZIP layer compression | Layer channels use PackBits then Deflate | `pictura-codec::tiff_layer_compression` |
| `tiff-discard-layers` | Write Discard Layers And Save A Copy | No layer section and the flattened image decodes | `pictura-codec::tiff_discard_layers` |
| `gif89a-signature` | Encode an Indexed fixture | `GIF89a`, a global colour table, and a trailer | `pictura-codec::gif89a_structure` |
| `gif-indexed-plane` | Encode and decode the retained palette and indices | Colours match without re-quantizing | `pictura-codec::gif_indexed_plane` |
| `gif-interlace-flag` | Encode interlaced and normal | Descriptor flag set only for interlaced, same pixels | `pictura-codec::gif_interlace_flag` |
| `gif-transparency` | Encode a palette with a transparent entry | Extension names the index and decoding keeps transparency | `pictura-codec::gif_transparent_index` |
| `tga-depth` | Write 16/24/32-bit TGA | Header depth matches and each decodes | `pictura-codec::tga_depth_decode` |
| `tga-rle` | Write uncompressed and RLE | Image type 2/10 and identical decoding | `pictura-codec::tga_rle_decode` |
| `tga-origin` | Write a TGA and decode it | Origin bits match the row order and the image is not flipped | `pictura-codec::tga_origin` |
| `raw-header` | Write header 0 and 16 | 16 zero bytes then samples; zero header starts with samples | `pictura-codec::raw_header_bytes` |
| `raw-interleave` | Write Interleaved and Non-interleaved | Pixel-interleaved then per-channel planes | `pictura-codec::raw_interleave_planes` |
| `raw-mode-channels` | Write Gray/RGB/CMYK/Lab | Sample count and order match the mode | `pictura-codec::raw_mode_channels` |
| `raw-multichannel-refused` | Write a Multichannel document as Raw | Failure, no file, no RGB fallback | `pictura-codec::raw_refuses_multichannel_without_planes` |
| `raw-eight-bit` | Write a document as Raw | Size is header + width × height × channels | `pictura-codec::raw_eight_bit_size` |
| `cineon-header` | Write a Cineon file | Magic and header width/height/channels match | `pictura-codec::cineon_header_magic` |
| `cineon-packing` | Decode a written Cineon file | Three 10-bit values per 32-bit word, size matches | `pictura-codec::cineon_10bit_packing` |
| `cineon-conversion` | Write the same document twice | Byte-identical, black low and white high | `pictura-codec::cineon_deterministic_mapping` |
| `cineon-no-alpha` | Write a document with transparency | Three channels and an opaque pixel | `pictura-codec::cineon_no_alpha` |
| `eps-dsc-header` | Write and parse an EPS | EPSF header and a bounding box matching the size; gs opens it | `pictura-codec::eps_dsc_header` |
| `eps-preview` | Write None/1-bit/8-bit previews | None is a plain EPS; each preview is a DOS EPS wrapper with a TIFF preview block | `pictura-codec::eps_preview_options` |
| `eps-encodings` | Write ASCII/ASCII85/Binary/JPEG | Each decodes and the JPEG form uses the quality | `pictura-codec::eps_encodings` |
| `eps-clipping-path` | Write with a named clipping path | The PostScript sets the clip before the image | `pictura-codec::eps_clipping_path` |
| `eps-vector-data` | Write with Include Vector Data and paths | PostScript path operators are present | `pictura-codec::eps_vector_data` |
| `eps-color-spaces` | Write Gray/RGB/CMYK and convert Lab/Multichannel/Indexed/Duotone/Bitmap sources | The device color space matches and the converted image decodes | `pictura-codec::eps_color_spaces` |
| `dcs-1-plates` | Write DCS 1.0 | Master plus four plate files, each valid EPS | `pictura-codec::dcs_1_plate_files` |
| `dcs-2-single` | Write DCS 2.0 | One master file carrying four process-color image blocks | `pictura-codec::dcs_2_single_file` |
| `dcs-process-only` | Write a document with extra channels | Process CMYK only and an omission report | `pictura-codec::dcs_process_only` |
| `dcs-preview` | Write DCS with None, 1-bit and 8-bit previews | The master is a DOS EPS wrapper whose TIFF preview matches the choice and the separations are unchanged | `pictura-codec::dcs_preview` |
| `raster-refuses-invalid` | Encode zero-size and short buffers | Failure, no file, no panic | `pictura-codec::save_refuses_invalid` |
| `raster-refuses-mode` | Request an unrepresentable mode | Failure, no file, previous target untouched | `pictura-codec::save_refuses_mode` |
| PNG preserves the composite alpha | Export a transparent pixel as PNG | The decoded pixel is transparent | `tst_save_options::exportEdgeScaleAlpha` |
| JPEG flattens alpha onto an opaque background | Export a transparent pixel as JPEG | The decoded pixel is opaque | `tst_save_options::exportEdgeScaleAlpha` |
| BMP flattens alpha onto an opaque background | Export a transparent pixel as BMP | The decoded pixel is opaque | `tst_save_options::exportEdgeScaleAlpha` |
| Scale resizes the output | Export a W × H document at 50 percent | The file is W/2 × H/2 and keeps its aspect ratio | `tst_save_options::exportEdgeScaleAlpha` |
| An unwritable target fails without a partial file | Export into a missing directory | Failure and no file | `tst_save_options::exportEdgeScaleAlpha` |
| `save-options-jpeg` | Resolve the options kind for `.jpg`, `.jpeg`, `.jpe` | JPEG Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-png` | Resolve the options kind for `.png` | PNG Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-bmp` | Resolve the options kind for `.bmp` | BMP Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-tiff` | Resolve the options kind for `.tif`, `.tiff` | TIFF Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-gif` | Resolve the options kind for `.gif` | GIF Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-eps` | Resolve the options kind for `.eps` with the EPS filter | EPS Options | `tst_save_format_options::formatKindForSuffix`, `tst_save_format_options::filterHintChoosesEps` |
| `save-options-dcs` | Resolve the options kind for `.eps` with the DCS filter | DCS Format | `tst_save_format_options::filterHintChoosesDcs` |
| `save-options-eps-dcs-hint` | Choose DCS, type `.eps`, reopen | DCS Format seeded from the DCS settings | `tst_save_format_options::filterHintChoosesDcs` |
| `save-options-raw` | Resolve the options kind for `.raw` | Photoshop Raw Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-tga` | Resolve the options kind for `.tga`, `.vda`, `.icb`, `.vst` | Targa Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-cineon` | Resolve the options kind for `.cin` | Cineon Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-native` | Resolve `.psd`, `.psb`, `.webp`, `.pdf` | No format dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-cancel` | Reject each options dialog through the test hook | No file, no state change | `tst_save_format_options::cancelWritesNothing` |
| `save-options-unknown-suffix` | Save to an unregistered suffix and to a path with no suffix | Failure and no file | `tst_save_format_options::unknownSuffixRefused`, `tst_save_format_options::missingSuffixRefused` |
| `save-options-defaults` | Open each dialog with no stored settings | The documented CS6 defaults are shown | `tst_save_format_options::dialogDefaults`, `tst_save_format_dialogs::gifControls`, `tst_save_format_dialogs::epsControls`, `tst_save_format_dialogs::dcsControls`, `tst_save_format_dialogs::rawControls`, `tst_save_format_dialogs::tgaControls`, `tst_save_format_dialogs::cineonControls` |
| `save-options-remembered` | Accept settings, reopen each dialog | The accepted settings are restored | `tst_save_format_options::rememberedSettings`, `tst_save_format_dialogs::gifControls`, `tst_save_format_dialogs::epsControls`, `tst_save_format_dialogs::dcsControls`, `tst_save_format_dialogs::rawControls`, `tst_save_format_dialogs::tgaControls`, `tst_save_format_dialogs::cineonControls` |
| `save-options-no-psd-worker` | Save a GIF/TGA/Raw/Cineon and then an unknown suffix through the app save paths | The new writer runs or the save is refused; the PSD worker is never used | `tst_save_format_options::saveStartRefusesNewFormats` |
| `save-options-feature-warning` | Save a layered document as GIF and as TGA | The feature-loss warning is shown before writing | `tst_save_format_options::featureWarningNewFormats` |
| `jpeg-quality` | Save at quality 12 and 2 | Quality 2 is smaller | `tst_save_format_options::jpegQualityChangesSize` |
| `jpeg-progressive` | Save Baseline and Progressive | SOF0 then SOF2 | `tst_save_format_options::jpegFormatOption` |
| `jpeg-matte-disabled` | Open JPEG Options and save a transparent pixel | Matte disabled with tooltip; pixel opaque | `tst_save_format_options::jpegMatte` |
| `png-compression` | Save at the lowest and highest compression | Sizes differ | `tst_save_format_options::pngCompressionChangesSize` |
| `png-interlace-option` | Save with Interlace None and Interlaced | IHDR interlace flag follows the option | `tst_save_format_options::pngInterlaceWrite` |
| `bmp-options` | Save Windows with each depth, RLE and Flip, and OS/2 at its supported depths | Header and pixels follow each choice; unsupported OS/2 combinations are refused | `tst_save_format_options::bmpOptionsWrite` |
| `tiff-compression` | Save with None, RLE, LZW, ZIP, JPEG | Tag 259 is 1/32773/5/8/7 | `tst_save_format_options::tiffCompressionTag` |
| `tiff-pixel-byte-order` | Save Per Channel and Macintosh | Tag 284 is 2 and the header is `MM` | `tst_save_format_options::tiffPixelByteOrderWrite` |
| `tiff-pyramid-option` | Save with Save Image Pyramid | SubIFDs exist and IFD0 decodes full size | `tst_save_format_options::tiffPyramidWrite` |
| `tiff-layer-compression-option` | Save a layered document with RLE, ZIP, Discard | Layer section present with the chosen compression, or flat | `tst_save_format_options::tiffLayerCompressionWrite` |
| `tiff-transparency` | Save a transparent pixel with Save Transparency off | Pixel is opaque | `tst_save_format_options::tiffTransparencyOffFlattens` |
| `eps-options` | Save with each EPS option changed | DSC header, preview and image invocation reflect it | `tst_save_format_options::epsOptionsWrite` |
| `raw-options` | Save with header 16 and both interleaves | Placeholder bytes and sample order follow | `tst_save_format_options::rawOptionsWrite` |
| `tga-options` | Save at 16/24/32 with RLE on/off | Header depth and image type follow | `tst_save_format_options::tgaOptionsWrite` |
| `cineon-options` | Save as Cineon | Three 10-bit channels and no 8-bit samples | `tst_save_format_options::cineonOptionsWrite` |
| `unsupported-disabled` | Open every dialog | Every unhonorable control is disabled with a tooltip | `tst_save_format_options::unsupportedControlsDisabled`, `tst_save_format_dialogs::bmpAdvancedControls`, `tst_save_format_dialogs::rawMacDisabled` |
| `raw-mac-disabled` | Open Photoshop Raw Options on Linux | File Type and File Creator disabled with a tooltip | `tst_save_format_dialogs::rawMacDisabled` |
| `save-options-gif-indexed` | Save an RGB document as GIF, accept both dialogs | Document becomes Indexed and the GIF uses the palette | `tst_save_format_options::gifSaveConverts` |
| `save-options-gif-cancel` | Run once cancelling Indexed Color, and once accepting it then cancelling GIF Options | Mode, pixels, path, dirty flag and history unchanged in both runs | `tst_save_format_options::gifCancelLeavesMode` |
| `gif-row-order` | Save Normal and Interlaced | GIF interlace flag follows the option | `tst_save_format_options::gifRowOrderWrite` |
| `save-options-dcs-1-files` | Save DCS 1.0 | Master and four plates exist and are valid | `tst_save_format_options::dcsFiles` |
| `save-options-dcs-2-single` | Save DCS 2.0 | Only the master exists and carries four separations | `tst_save_format_options::dcsFiles` |
| `dcs-spot-ceiling` | Save a document with extra channels | Process CMYK only and a warning | `tst_save_format_options::dcsSpotCeiling` |
| `dcs-atomic-set` | Fail one plate write | No master or plate remains | `tst_save_format_options::dcsFiles` |
| `save-options-plain-kind` | Save through DCS, then plain Save | The second write is DCS, not EPS | `tst_save_format_options::plainSaveKind` |
| `save-options-plain-remembered` | Save `.tga`, reopen, plain Save | Remembered TGA options are used | `tst_save_format_options::plainSaveKind` |
| Photoshop layered-TIFF reopen | Open a layered TIFF in Photoshop and compare layers and pixels | Same layer count and pixel-identical layers; other applications see the flattened composite | manual: requires a Photoshop install, unavailable in CI |
| DCS page-layout interop | Place a DCS 1.0 set in a page-layout application | Plates composite correctly | manual: requires a page-layout application, unavailable in CI |
| Cineon film-tool interop | Open the Cineon file in a film tool | RGB frames decode at the document size | manual: requires a Cineon film tool, unavailable in CI |
| CS6 visual comparison | Compare each dialog and written file against CS6 on a reference install | Controls, defaults and output match the documented approximations | manual: requires a Photoshop CS6 install, unavailable in CI |