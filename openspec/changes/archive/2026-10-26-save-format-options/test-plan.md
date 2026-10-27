# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `save-options-jpeg` | resolve the options kind for `.jpg`, `.jpeg`, `.jpe` | each resolves to the JPEG dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-png` | resolve the options kind for `.png` | resolves to the PNG dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-bmp` | resolve the options kind for `.bmp` | resolves to the BMP dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-tiff` | resolve the options kind for `.tif`, `.tiff` | resolves to the TIFF dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-native` | resolve the options kind for `.psd`, `.psb`, `.webp`, `.pdf` | each resolves to no format dialog | `tst_save_format_options::formatKindForSuffix` |
| `save-options-cancel` | reject the format dialog through the frame's test hook and call the Save-As-with-options entry | it returns false and no file is written | `tst_save_format_options::cancelWritesNothing` |
| `save-options-defaults` | construct each dialog with no stored settings | JPEG 10/Baseline, PNG None/Fast, TIFF LZW, BMP Windows/24 | `tst_save_format_options::dialogDefaults` |
| `save-options-remembered` | accept settings through the hook, then reload the session and open the dialog again | the controls hold the accepted settings | `tst_save_format_options::rememberedSettings` |
| `jpeg-quality` | save the same document at quality 12 and quality 2 through the view's stored options | the quality-2 file is smaller | `tst_save_format_options::jpegQualityChangesSize`, `tst_save_format_options::jpegQualityMapping` |
| `jpeg-progressive` | save with Baseline and with Progressive | Baseline carries SOF0, Progressive carries SOF2 | `tst_save_format_options::jpegFormatOption` |
| `jpeg-matte-disabled` | open the JPEG Options dialog | the Matte control is disabled with a tooltip, and a transparent pixel saves as opaque | `tst_save_format_options::unsupportedControlsDisabled`, `tst_save_format_options::jpegMatte` |
| `png-compression` | save the same document at compression 0 and 9 | the two files differ in size | `tst_save_format_options::pngCompressionChangesSize` |
| `tiff-compression` | save with None and with LZW compression | the written TIFF's tag 259 is 1 and 5 | `tst_save_format_options::tiffCompressionTag` |
| `tiff-transparency` | save a document with a transparent pixel with Save Transparency off | the written TIFF's pixel is opaque | `tst_save_format_options::tiffTransparencyOffFlattens` |
| `unsupported-disabled` | construct each dialog | every control the encoder cannot honor is disabled and carries a tooltip | `tst_save_format_options::unsupportedControlsDisabled` |
| `jpeg-size-estimate` | lower the quality in the JPEG dialog | the file-size readout decreases | `tst_save_format_options::sizeEstimateFollowsQuality` |
| `jpeg-preview-no-mutation` | toggle Preview in the JPEG dialog | the document's pixels, path, modified state, and history are unchanged | `tst_save_format_options::previewDoesNotMutate` |
| manual: real file-dialog flow | run `File ▸ Save As` in a real session, choose each format, confirm the options dialog appears and the written file opens | each dialog appears and the file is written | manual: the `QFileDialog` is modal and not driven offscreen |
| manual: CS6 layout comparison | open each dialog and compare against the CS6 screenshots in the issue | the control set and grouping match | manual: visual parity, no oracle |
