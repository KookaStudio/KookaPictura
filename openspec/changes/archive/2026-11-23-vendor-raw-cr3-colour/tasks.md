# Tasks

## 1. Tests
- [x] 1.1 `tests/camera_raw/raw_cr3_colour.rs`: camera-JPEG lookup, white and decode tests; the CR3 corpus test accepts a one-matrix row

## 2. Tools and table
- [x] 2.1 `fit-camera-jpeg-matrices.py` reads CR3 (JPEG track, CMT1, CTMD presets) and keeps fits of samples outside the run
- [x] 2.2 `gen-camera-matrices.py` adds camera-JPEG proxies; the fit script drops a tungsten matrix whose pair does not read back its lights
- [x] 2.3 Fit the CR3 samples and regenerate `camera_matrices.rs`

## 3. Close
- [x] 3.1 Follow-up issue #1579 for the R3, the PowerShot V1, the R6 Mark III pair and the RP / R50 rows
- [x] 3.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 3.3 Archive the change
