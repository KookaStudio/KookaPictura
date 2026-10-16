# Tasks: clone-stamp-tool

## 1. Engine

- [x] 1.1 `pictura_paint::stamp` (`StampSource`, `CloneSampling`, `sample_scope`, `layer_surface`, `surface_from_composite`); unit tests.
- [x] 1.2 `Stroke::begin_source`; an alpha-less layer reads as opaque; unit tests.

## 2. Bridge

- [x] 2.1 `begin_clone_stamp` in `cxxqt_object/paint_tools.rs`; one "Clone Stamp" state.

## 3. Tool

- [x] 3.1 `tool_stamps.cpp` Clone Stamp handler (Alt-click source, Aligned, refusals); catalog row enabled.
- [x] 3.2 Options-bar row: paint fields, Aligned, Sample, Ignore Adjustment Layers.

## 4. Verification

- [x] 4.1 `clone_stamp_tool` self-test (542); `shift_plain` (117), `keys_shown` (116), guard (98) updated; `bash scripts/verify-fast.sh`.
