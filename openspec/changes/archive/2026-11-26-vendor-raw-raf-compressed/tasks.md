# Tasks

## 1. Tests
- [x] 1.1 Test encoder for compressed data (`tests/camera_raw/raw/raf_encode.rs`)
- [x] 1.2 Every test named in `test-plan.md`

## 2. Codec
- [x] 2.1 `dng/raf_compressed.rs`: header, strip table, quantizer bases, line decode
- [x] 2.2 `dng/raf.rs`: detect the header, take its size, dispatch

## 3. Close
- [x] 3.1 Corpus run against LibRaw (`PICTURA_RAW_CORPUS`, rawpy)
- [x] 3.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 3.3 Archive the change
