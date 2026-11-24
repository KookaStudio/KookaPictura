# Tasks

## 1. Codec
- [x] 1.1 `Tiff::words` reads SHORT values or BYTE / UNDEFINED byte pairs; the CR2 / CR3 as-shot reader uses it

## 2. Tools and table
- [x] 2.1 `fit-camera-jpeg-matrices.py` reads CR2s and colour-data version -4 (8-word records)
- [x] 2.2 Fit the owner's samples; regenerate the tables; remove the camera-JPEG proxies from `gen-camera-matrices.py`

## 3. Tests
- [x] 3.1 CR2 builder writes colour data as SHORT, UNDEFINED or LONG; `raw_cr2.rs` covers bytes, version -4 and a LONG entry
- [x] 3.2 `raw_cr3_colour.rs`: R8 Mark II, M6 Mark II and G7 X Mark III as camera-JPEG bodies; no proxies; EOS Ra colourless
- [x] 3.3 `raw_cr3.rs`: the colourless CR3 lookup names the EOS Ra
- [x] 3.4 `raw_camera_matrices.rs`: the G1 X Mark III CR2 takes its D55 row, keeps white, and decodes

## 4. Close
- [x] 4.1 Follow-up issue for the CR2 white level (#1587)
- [x] 4.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 4.3 Archive the change
