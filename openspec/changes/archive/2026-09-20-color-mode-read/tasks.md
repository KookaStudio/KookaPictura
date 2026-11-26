## 1. Model

- [x] 1.1 Add `pub source_mode: Option<ColorMode>` to `pictura_core::Document`; set it to `None` in `Document::new`/`Document::from_rgba` and in any struct literal that lists every field (`..Default::default()` covers the rest until `cargo check --workspace --all-targets` is clean).
- [x] 1.2 Confirm `ColorMode::color_channels()` already encodes the mode→channel mapping (Bitmap/Grayscale/Indexed/Duotone 1, RGB/Lab 3, CMYK 4); no change unless a gap is found.

## 2. Codec: header, depth, and rows

- [x] 2.1 In `crates/pictura-codec/src/common.rs`, add the mode constants `MODE_BITMAP`, `MODE_INDEXED`, `MODE_CMYK`, `MODE_LAB` (and, if the header gate references them, `MODE_MULTICHANNEL`, `MODE_DUOTONE`).
- [x] 2.2 In `read.rs`, replace the mode/depth gate with the `psd-color-modes` mapping: accept modes 0/1/2/3/4/9, accept depth 1 only for Bitmap, keep 16/32 and modes 7/8 as `PsdError::Unsupported`.
- [x] 2.3 Thread a depth-aware row stride (`ceil(width * bits / 8)`) through the raw branch, `read_rle`, and `planar_len`; expand a depth-1 layer channel to an 8-bit plane in `read_channel_data` so the layer path matches the composite; return `PsdError::Unsupported` for compression 2/3 at depth 1.
- [x] 2.4 Keep the header channel-count check (`header_channels >= mode.color_channels()`), and leave extra alpha/spot/selection planes unconverted.

## 3. Codec: color conversion

- [x] 3.1 Add `crates/pictura-codec/src/color_mode.rs` with pure functions: `bitmap_rows_to_rgb` (MSB-first, `0x80 >> (x % 8)`, set = black), `indexed_to_rgb` (768-byte non-interleaved palette, `usize` bounds-checked), `cmyk_to_rgb` (`floor(c*k/255)`), and `lab_to_rgb` (CIELAB D50 → sRGB via the D3/D6 matrix), each with a `ponytail:` comment naming the approximation or the gamut-clipping ceiling.
- [x] 3.2 Parse an Indexed color-mode-data section as the palette; a length other than 768 returns `PsdError::Invalid`.
- [x] 3.3 Convert the composite's color planes to RGB after `split_planes`; convert each layer's color channels (ids `0..color_channels`) with the same function and leave `-1`/`-2`/unmodeled channels untouched. A layer whose color channels do not match the mode layout is left unchanged.
- [x] 3.4 On a normalized document set `mode = Rgb`, `depth = Eight`, `source_mode = Some(header_mode)`, and clear `color_mode_data` only for Indexed; leave Grayscale/RGB read exactly as before.

## 4. Codec unit tests

- [x] 4.1 `crates/pictura-codec/src/tests.rs`: Bitmap packing (`0xAA` row → black/white pattern), a non-multiple-of-8 width, palette index lookup, the CMYK known value `(128,64,32,200) → (100,50,25)`, full-black/full-white, and the Lab neutral-gray/white values within 2 plus a non-neutral table matching lcms2's exact transform.
- [x] 4.2 Malformed inputs: a 767/769-byte palette (`Invalid`), a depth-1 ZIP (`Unsupported`), a non-RGB layer with a mismatched channel count, and a truncated depth-1 RLE row — all typed errors, never a panic.
- [x] 4.3 `source_mode`/`color_mode_data` assertions: CMYK/Indexed/Bitmap/Lab read back with the right `source_mode`; RGB/Gray and constructed documents have `None`.

## 5. Fixtures and oracle

- [x] 5.1 In `scripts/generate-fixtures.py`, add `indexed()`, `cmyk()`, `lab()`, and `bitmap()` builders (design D8) and register them in `FIXTURES`; the `bitmap()` builder mutates `header.depth = 1` before `set_data`. Confirm the CMYK fixture carries one pixel layer to exercise layer conversion, and that regeneration is byte-stable and additive.
- [x] 5.2 Commit `crates/pictura-codec/tests/fixtures/{indexed,cmyk,lab,bitmap}.psd`.
- [x] 5.3 Add a psd-tools oracle (extend `tests/oracle.rs` or add `tests/psd/color_mode_oracle.rs`) that decodes each fixture with `psd-tools` (composite/indices), compares to `read_psd` (Indexed/Bitmap exact, CMYK within the 1-LSB rounding, Lab within 1 against lcms2's exact `Flags.NOOPTIMIZE` transform rather than psd-tools' optimized `.convert("RGB")`), and self-skips with a clear message when psd-tools is absent.
- [x] 5.4 Add a pure-Rust hand-built depth-1 Bitmap **RLE** file test (the P1 `one_layer_zip_psd` precedent) and assert the decoded pixels against the hand-computed pattern.
- [x] 5.5 Add a round-trip test: read each fixture, assert `mode`/`source_mode`/empty palette for Indexed; write with `write_psd`, re-read, and assert the normalized document is stable and the output's header mode is RGB; confirm psd-tools opens the output as RGB.
- [x] 5.6 Confirm no existing fixture or golden changes.

## 6. App

- [x] 6.1 In `crates/pictura-app/src/cxxqt_object.rs`, add `#[qinvokable] fn mode_notice(&self) -> QString` returning `"Converted from CMYK"` / `"... Lab"` / `"... Indexed"` / `"... Bitmap"` from `doc.source_mode` (empty otherwise), plus `#[qinvokable] fn document_mode(&self) -> QString` returning `"rgb"` / `"grayscale"` / empty for the working mode.
- [x] 6.2 In `crates/pictura-app/cpp/frame.cpp::openPath`, after a successful `view->open(path)`, show `view->mode_notice()` in `statusBar()` when non-empty.
- [x] 6.3 Add one C++ self-test check (`selftest_color_modes.cpp`/`.h`, registered in `CMakeLists.txt` and the `runSelfTest` dispatch) taking **exit code 297** (295 = `present_cache_edge`, 296 = `lpr_selective_color`): write a minimal flat CMYK PSD to a `QTemporaryDir`, `view->open` it, and assert the open succeeds, the notice names CMYK, and the view's document mode reads RGB. (Implemented as `color_mode_open` in the existing `selftest_layers_adjustments.cpp` sub-runner instead of a new `.cpp`/`.h`, to avoid a `CMakeLists.txt` edit and a concurrent-session collision.)
- [x] 6.4 Confirm `new_document` and `decode_image`/`open_image` are unchanged (creation still offers only RGB/Grayscale).

## 7. Gates

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 7.2 `bash scripts/verify-full.sh` and a headless self-test; record counts and confirm the psd-tools oracle ran rather than self-skipped.
- [x] 7.3 `openspec validate color-mode-read --strict` and `openspec validate --all --strict`.
- [x] 7.4 Commit code + openspec + fixtures. `docs/` updates (roadmap G2/G3, `STATE.md`) stay a separate `TASK-ALLOWS-DOCS` commit. (Deferred: the apply task instructs no commit.)
