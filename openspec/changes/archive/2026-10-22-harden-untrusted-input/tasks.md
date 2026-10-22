# Tasks

- [x] Rust tests: `hostile_inputs.rs` (crafted PSD/PSB, budget, nesting, PSD and PDF mutation passes), `pdf/import/hostile_tests.rs`, `pdf/read.rs` tests, `png.rs` and `crypt.rs` cases, Ghostscript bound tests, `budget.rs` tests, `safe_io.rs` tests, `psd_rect_extent_saturates_instead_of_overflowing`
- [x] Qt Test: `tst_untrusted_input` (control token, Qt decode budget, thumbnail unpack)
- [x] Decode budget and group-nesting cap in the PSD reader; decoders moved to `read/decode.rs`
- [x] PDF stream, predictor, object-stream, lexer, page-tree, and AES bounds
- [x] PDF import page and image caps, render and crop budget, and bounded PNG decode
- [x] Ghostscript timeout, rlimits, output caps, minimum version, and separator-safe input
- [x] Qt allocation limit and size pre-check, capped raster reads, and checked thumbnail unpacking
- [x] Control session token, and the self-test and script clients
- [x] Atomic saves through `safe_io`, and OS entropy
