# Tasks

## 1. Tests first

- [x] 1.1 Add `qtDecodeRefusesShortInput`, `qtDecodePinsKnownSignatures`, and `qtDecodeStillProbesUnknownSignatures` to `tst_untrusted_input`.
- [x] 1.2 Add `emptyAndStubImagesAreRefused` to `tst_multi_file_open`, driving File ▸ Open with a 0-byte and a 3-byte `.jpg`.

## 2. Decode edge

- [x] 2.1 Refuse input under 16 bytes in `decode_image_rgba` before the `QBuffer` and `QImageReader` are built.
- [x] 2.2 Add `pictura::qtFormatForSignature` and pin the reader's format, with auto-detect off, for PNG, JPEG, GIF, BMP, and TIFF.

## 3. Verification

- [x] 3.1 Build once, then run both Qt suites and `openspec validate --all --strict`.
