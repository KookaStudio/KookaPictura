# Tasks

## 1. Tests
- [x] 1.1 Test encoder for compressed streams (`tests/camera_raw/raw/orf_encode.rs`)
- [x] 1.2 Every test named in `test-plan.md`

## 2. Codec
- [x] 2.1 `dng/orf_decode.rs`: parameters, header, bit pump, predictor
- [x] 2.2 `dng/orf_meta.rs`: read ImageProcessing `0x0640`-`0x0653`
- [x] 2.3 `dng/orf.rs`: dispatch the compressed coding, 14-bit level scale

## 3. Close
- [x] 3.1 Corpus run against LibRaw (`PICTURA_RAW_CORPUS`, rawpy)
- [x] 3.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 3.3 Archive the change
