# Tasks

## 1. Tests
- [x] 1.1 `tests/camera_raw/raw_cr3_colour.rs`: R1, R3, PowerShot V1 and the R6 Mark III pair in the camera-JPEG lookup and white tests; G1 X Mark III and R8 Mark II without colour
- [x] 1.2 `tests/camera_raw/raw_cr3.rs`: the colourless lookup and camera-tag cases name the PowerShot G1 X Mark III instead of the R3

## 2. Tools and table
- [x] 2.1 `fit-camera-jpeg-matrices.py` reads colour-data version 67
- [x] 2.2 `gen-camera-matrices.py` prefers a body's sample with both matrices
- [x] 2.3 Fit the owner's samples and regenerate the table, its camera-JPEG rows split into `camera_jpeg_matrices.rs` (size cap)
- [x] 2.4 Check the RP and R50 spectral rows against Canon's presets; keep both

## 3. Close
- [x] 3.1 Follow-up issue for the R8 Mark II and the bodies without a usable sample
- [x] 3.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 3.3 Archive the change
