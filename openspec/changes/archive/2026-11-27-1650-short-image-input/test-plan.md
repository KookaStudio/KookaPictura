# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `image-import-empty-input` | Call `decode_image_rgba` with an empty slice. | Empty buffer, width and height 0, no crash. | `tst_untrusted_input::qtDecodeRefusesShortInput` |
| `image-import-short-input` | Call `decode_image_rgba` with 3 bytes, then with the first 15 bytes of a valid PNG. | Both refused with a zero size. | `tst_untrusted_input::qtDecodeRefusesShortInput` |
| `image-import-pinned-format` | Map each of the five signatures with `qtFormatForSignature`; decode a valid PNG; decode a PNG signature followed by 24 garbage bytes. | Names `png`/`jpeg`/`gif`/`bmp`/`tiff`; the PNG decodes; the garbage is refused. | `tst_untrusted_input::qtDecodePinsKnownSignatures` |
| `image-import-unknown-probed` | Map and decode a 23-byte plain PBM. | No pinned format; it decodes to 4 × 2. | `tst_untrusted_input::qtDecodeStillProbesUnknownSignatures` |
| `image-import-open-short-file` | Drive File ▸ Open with `empty.jpg` (0 bytes), then `stub.jpg` (3 bytes). | No document opens, the status bar shows `Could not open`, and the window is still alive. | `tst_multi_file_open::emptyAndStubImagesAreRefused` |
