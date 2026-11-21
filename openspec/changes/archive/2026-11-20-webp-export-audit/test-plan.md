# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ce_export_webp_options` | Call `export_webp` with quality 101, quality -1, a fractional quality, a string quality, and a non-boolean lossless. | Each response is `invalid_param` and the destination does not exist. | `tst_control_editing::exportWebpRejectsBadOptions` |
| `webp_quality_range` | Select WebP with Lossless unchecked, inspect the Quality range, and encode at 0 and 100. | The range is 0 through 100 and both encodings contain a VP8 stream. | `tst_export_as::webpQualityRange` |
