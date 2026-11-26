# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pict-read-direct32` | Decode a synthetic 32-bit `DirectBitsRect` picture | Pixels and dimensions match the source | `pictura-codec::pict_read_direct32` |
| `pict-read-direct16` | Decode a synthetic 16-bit RGB555 picture | Five-bit channels expand to the expected eight-bit values | `pictura-codec::pict_read_direct16` |
| `pict-read-indexed` | Decode a synthetic 8-bit indexed `PackBitsRect` with a colour table | Each pixel is its table entry | `pictura-codec::pict_read_indexed` |
| `pict-read-bitmap` | Decode a synthetic one-bit `BitsRect` and `PackBitsRect` | Zero is the background colour and one the foreground | `pictura-codec::pict_read_bitmap` |
| `pict-read-v1` | Decode a synthetic version 1 one-bit picture | One-byte opcodes are read and the bitmap decodes | `pictura-codec::pict_read_v1` |
| `pict-read-multiple` | Decode a picture with two image records at different rectangles | Both records replace the canvas at their rectangles | `pictura-codec::pict_read_multiple` |
| `pict-read-jpeg` | Decode a synthetic `$8200` record with a real JPEG | The JPEG pixels appear at the record rectangle | `pictura-codec::pict_read_jpeg` |
| `pict-read-alpha` | Decode a 32-bit record with four components | The decoded alpha matches | `pictura-codec::pict_read_alpha` |
| `pict-read-packbits` | Decode rows with literal and repeated PackBits runs | The runs expand to the exact row bytes | `pictura-codec::pict_read_packbits` |
| `pict-refuse-malformed` | Truncate the header, a record and its pixel data | Each returns a malformed error without panicking | `pictura-codec::pict_refuse_malformed` |
| `pict-refuse-vector` | Decode a picture with only drawing records | Unsupported error naming the missing bitmap | `pictura-codec::pict_refuse_vector` |
| `pict-refuse-unsupported` | Decode a record with an unsupported pixel format, packing type, pattern or `$8201` | Unsupported error naming the feature | `pictura-codec::pict_refuse_unsupported` |
| `pict-refuse-oversized` | Declare a canvas and a record over the budget | Refused before allocation | `pictura-codec::pict_refuse_oversized` |
| `pict-refuse-jpeg` | Put a non-JPEG payload in a `$8200` record | JPEG error without panicking | `pictura-codec::pict_refuse_jpeg` |
| `pict-write-32` | Write an opaque image at 32 bits and decode it | Pixels and dimensions match | `pictura-codec::pict_write_32` |
| `pict-write-16` | Write an image at 16 bits and decode it | Pixels match within five-bit quantisation | `pictura-codec::pict_write_16` |
| `pict-write-alpha` | Write a transparent pixel at 32 bits and decode it | The decoded pixel is transparent | `pictura-codec::pict_write_alpha` |
| `pict-write-deterministic` | Write the same image and options twice | Byte-identical output | `pictura-codec::pict_write_deterministic` |
| `pict-write-jpeg` | Write each JPEG level and decode the result | `$8200` present, image decodes, lower levels are no larger | `pictura-codec::pict_write_jpeg` |
| `pict-write-limits` | Request widths/heights over the field or row limit | Refused without a file | `pictura-codec::pict_write_limits` |
| A recognized header yields declared metadata | Probe PNG/JPEG/GIF/BMP/TIFF/WebP/PICT headers | Each returns its format and declared size | `pictura-codec::recognizes_each_container_header` |
| `image-import-pict` | Probe a PICT file with the version opcode at 522 and at 10 | `ImageFormat::Pict` and the frame size | `pictura-codec::recognizes_a_pict_header` |
| A declared dimension over the maximum is refused | Probe a header over the dimension budget | Dimension-limit error | `pictura-codec::refuses_dimension_over_budget_naming_the_limit` |
| An estimated allocation over budget is refused | Probe a header over the allocation budget | Allocation-limit error | `pictura-codec::refuses_allocation_over_budget_naming_the_limit` |
| An unrecognized container is reported as unknown | Probe arbitrary bytes | `UnknownContainer`, no panic | `pictura-codec::refuses_unknown_and_truncated_without_panicking` |
| A truncated header is refused | Probe a recognized header cut before its dimensions | Error, no panic | `pictura-codec::refuses_unknown_and_truncated_without_panicking` |
| A supported file decodes to packed RGBA8888 | Import a supported raster | Packed RGBA8888 at the decoded size | `tst_image_import::pngOff` |
| `image-import-pict-decode` | Import a committed PICT fixture | The document opens with the fixture's pixels and Qt is not the decoder | `tst_image_import::pictOpens` |
| A Qt-decodable container unknown to the probe is imported | Import a container the probe does not know | Qt decodes and the import succeeds | `tst_image_import::untagged` |
| An undecodable file is refused | Import bytes neither engine nor Qt can read | Failure, no document | `tst_image_import::malformedProfileIgnored` |
| The actual decoded allocation is capped | Import a file whose declared header fits but whose decoded allocation exceeds the budget | Refused before engine structures | `tst_untrusted_input::qtDecodeRefusesSizesOverTheBudget` |
| The decoded rows are copied in bulk | Decode an image at the import edge | Bulk row copies, no per-byte bridge loop | manual: a performance property of the existing decode edge; there is no per-byte loop to assert |
| Only the first frame is imported | Decode a multi-frame GIF/APNG/WebP | Frame 0 only | manual: the engine WebP decoder takes the first frame; no animated fixture is committed |
| `open_as_command` | Read the File menu | Open As is enabled with its shortcut | `tst_import_pdf::openAsIsLive` |
| `open_as_forces_format` | Open a PNG named `.dat` with the PNG filter | The PNG pixels open | `tst_import_pdf::openAsForcesFormat` |
| `open-as-pict` | Open a PICT with the PICT filter; open a non-PICT with it | The PICT opens; the other reports a mismatch | `tst_import_pdf::openAsPict` |
| `open_as_mismatch` | Open a PNG with the Generic PDF filter | Mismatch status, no document | `tst_import_pdf::openAsReportsMismatch` |
| `open_filters_pdf_eps` | Read the Open filters | Generic PDF and EPS entries exist | `tst_import_pdf::openAsIsLive` |
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
| `save-options-pict` | Resolve the options kind for `.pct`, `.pic`, `.pict` | PICT Options | `tst_save_format_options::formatKindForSuffix` |
| `save-options-native` | Resolve `.psd`, `.psb`, `.webp`, `.pdf` | No format dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-cancel` | Reject each options dialog through the test hook | No file, no state change | `tst_save_format_options::cancelWritesNothing` |
| `save-options-unknown-suffix` | Save to an unregistered suffix and to a path with no suffix | Failure and no file | `tst_save_format_options::unknownSuffixRefused`, `tst_save_format_options::missingSuffixRefused` |
| `save-options-defaults` | Open each dialog with no stored settings | The documented CS6 defaults are shown | `tst_save_format_options::dialogDefaults`, `tst_save_format_dialogs::pictControls` |
| `save-options-remembered` | Accept settings, reopen each dialog | The accepted settings are restored | `tst_save_format_options::rememberedSettings`, `tst_save_format_dialogs::pictControls` |
| `save-options-pict-remembered` | Accept 16-bit + JPEG High in PICT Options and reopen | The same settings are shown | `tst_save_format_options::pictRemembered` |
| `jpeg-quality` | Save at quality 12 and 2 | Quality 2 is smaller | `tst_save_format_options::jpegQualityChangesSize` |
| `jpeg-progressive` | Save Baseline and Progressive | SOF0 then SOF2 | `tst_save_format_options::jpegFormatOption` |
| `jpeg-matte-disabled` | Open JPEG Options and save a transparent pixel | Matte disabled with tooltip; pixel opaque | `tst_save_format_options::jpegMatte` |
| `png-compression` | Save at the lowest and highest compression | Sizes differ | `tst_save_format_options::pngCompressionChangesSize` |
| `png-interlace-option` | Save with Interlace None and Interlaced | IHDR interlace flag follows the option | `tst_save_format_options::pngInterlaceWrite` |
| `bmp-options` | Save Windows with each depth, RLE and Flip, and OS/2 at its supported depths | Header and pixels follow each choice | `tst_save_format_options::bmpOptionsWrite` |
| `tiff-compression` | Save with None, RLE, LZW, ZIP, JPEG | Tag 259 is 1/32773/5/8/7 | `tst_save_format_options::tiffCompressionTag` |
| `tiff-pixel-byte-order` | Save Per Channel and Macintosh | Tag 284 is 2 and the header is `MM` | `tst_save_format_options::tiffPixelByteOrderWrite` |
| `tiff-pyramid-option` | Save with Save Image Pyramid | SubIFDs exist and IFD0 decodes full size | `tst_save_format_options::tiffPyramidWrite` |
| `tiff-layer-compression-option` | Save a layered document with RLE, ZIP, Discard | Layer section present with the chosen compression, or flat | `tst_save_format_options::tiffLayerCompressionWrite` |
| `tiff-transparency` | Save a transparent pixel with Save Transparency off | Pixel is opaque | `tst_save_format_options::tiffTransparencyOffFlattens` |
| `eps-options` | Save with each EPS option changed | DSC header, preview and image invocation reflect it | `tst_save_format_options::epsOptionsWrite` |
| `raw-options` | Save with header 16 and both interleaves | Placeholder bytes and sample order follow | `tst_save_format_options::rawOptionsWrite` |
| `tga-options` | Save at 16/24/32 with RLE on/off | Header depth and image type follow | `tst_save_format_options::tgaOptionsWrite` |
| `cineon-options` | Save as Cineon | Three 10-bit channels and no 8-bit samples | `tst_save_format_options::cineonOptionsWrite` |
| `pict-options` | Save at 16 and 32 bits with None and each JPEG level | DirectBits or `$8200` follows the choice; 32-bit None keeps alpha | `tst_save_format_options::pictOptionsWrite` |
| `unsupported-disabled` | Open every dialog | Every unhonorable control is disabled with a tooltip | `tst_save_format_options::unsupportedControlsDisabled` |
| `raw-mac-disabled` | Open Photoshop Raw Options on Linux | File Type and File Creator disabled with a tooltip | `tst_save_format_dialogs::rawMacDisabled` |
| PICT round trip through ImageMagick | Write a PICT and decode it with `magick`; write one with `magick` and decode it in the engine | Both directions agree at the image size | `pictura-codec::magick_decodes_pict`, `pictura-codec::pict_decodes_magick_output` |
| Photoshop CS6 PICT round trip | Open a PICT written by Pictura in Photoshop CS6, and a CS6 PICT in Pictura | Pixels, dimensions and resolution agree; vector-only PICTs are refused | manual: requires a Photoshop CS6 install, unavailable in CI |
| PICT drawing-record behaviour | Open a vector-only PICT and a bitmap PICT in Pictura | The vector-only file reports unsupported; the bitmap file opens | manual: a public vector-only fixture is not in the repository |
