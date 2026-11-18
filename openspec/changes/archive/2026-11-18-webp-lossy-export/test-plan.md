# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| PNG-8 writes an indexed image | Export a colourful entry as PNG-8 and decode the bytes. | The image has the target size and at most 256 distinct colours. | `tst_export_as::png8AndGif` |
| GIF writes a GIF89a file | Export the entry as GIF and inspect the header. | The file starts with GIF89a and decodes to the target size. | `tst_export_as::png8AndGif` |
| `webp_quality_controls` | Select WebP, inspect the controls, uncheck and check Lossless, then select JPG and PNG. | WebP enables Quality and Transparency; Lossless disables Quality; JPG enables Quality; PNG disables it. | `tst_export_as::webpQualityControls` |
| `webp_lossy_default` | Select WebP without touching the controls and encode an entry. | Lossless is unchecked, Quality is 90 and enabled, and the bytes contain a VP8 stream. | `tst_export_as::webpQualityControls` |
| `webp_lossless_control` | Check Lossless on WebP, encode, write the file, and reopen it. | Quality is disabled, the file contains VP8L, and the reopened pixels equal the source. | `tst_export_as::webpQualityControls`, `tst_webp::exportLosslessRoundTrips` |
| The full format list is offered | Open the Format combo. | It contains PNG, JPG, PNG-8, GIF, SVG, and WebP. | `tst_export_as::formatsAndControls` |
| WebP export preserves alpha | Export a picture with a clear corner as lossy and as lossless WebP, then reopen each file. | The corner stays transparent in both files. | `tst_webp::exportLossyPreservesAlpha`, `tst_webp::exportLosslessRoundTrips` |
| WebP save round-trips through the engine | Save a patterned document to `.webp` and reopen it. | Every pixel matches the document; Save As did not take the lossy path. | `tst_webp::saveStaysLossless` |
| `webp_lossy_smaller` | Encode the same gradient lossy at 90 and lossless, and compare the byte counts. | Lossy is smaller, and both decode to the target dimensions. | `pictura-codec::lossy_webp_is_smaller_than_lossless`, `tst_export_as::webpLossyIsSmallerThanLossless` |
| `webp_lossy_quality` | Encode the same gradient at quality 10 and 90. | The low-quality file is smaller. | `pictura-codec::lossy_quality_orders_file_size`, `tst_export_as::webpLossyIsSmallerThanLossless` |
| `webp_lossy_alpha` | Encode a picture with a transparent corner at quality 80 and decode it. | The corner alpha is 0 and opaque pixels stay 255. | `pictura-codec::lossy_webp_preserves_alpha` |
| A malformed buffer is refused | Call both encoders with a zero dimension and with a short buffer. | Both return `None` without panicking. | `pictura-codec::lossy_webp_refuses_a_bad_buffer`, `pictura-codec::encode_refuses_a_zero_dimension_or_short_buffer` |
| Independent decoder accepts our lossy output | Encode a gradient with `encode_webp_lossy`, write it, and decode it with ImageMagick. | The decoded pixels are within the lossy tolerance; the check self-skips without `magick`. | `pictura-codec::magick_decodes_our_lossy_webp_within_tolerance` |
