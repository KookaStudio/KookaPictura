# Tasks

## 1. Tests

- [x] 1.1 Add `an_extra_channel_saves_without_a_duplicate_alpha_plane` in `crates/pictura-render/src/document_ops/mode/tests.rs`: build the `Document::from_rgba` base, then set `doc.composite = pictura_core::PixelBuffer::new(0, 0, 4)` (keeping `merged_composite_present` true) so `save_view` exercises its `composite_tiled` branch, the real app state for a layered 8-bit RGB document; give it one `doc.channels` alpha whose bytes differ from the display alpha; write through `save_view` and read back. Verify `cargo test -p pictura-render an_extra_channel_saves_without_a_duplicate_alpha_plane` fails before the fix (header 5) and passes after.
- [x] 1.2 Add `an_rgba_document_without_extra_channels_keeps_its_alpha_plane` in the same file: a `Document::from_rgba` with non-opaque alpha and empty `doc.channels`; write and read back. Verify `cargo test -p pictura-render an_rgba_document_without_extra_channels_keeps_its_alpha_plane` passes before and after.
- [x] 1.3 Extend and rename `save_view_trims_the_display_alpha_only_for_a_source_mode` to `save_view_trims_the_display_alpha_for_a_source_mode_or_extra_channels`, adding the RGB-composite-plus-extra case. Verify `cargo test -p pictura-render save_view_trims_the_display_alpha_for_a_source_mode_or_extra_channels` fails before the fix and passes after.
- [x] 1.4 Add `a_depth16_extra_channel_saves_without_a_duplicate_alpha_plane` in the same file: a 2x2 layered RGB/16 fixture with `source_depth = Some(BitDepth::Sixteen)`, a four-plane `SourcePlanes` store, and one document extra channel; save through `save_view` and read back. Assert the header declares 4 channels at depth 16 and the re-read `channels[0].data` equals the extra. Verify `cargo test -p pictura-render a_depth16_extra_channel_saves_without_a_duplicate_alpha_plane` fails before the fix and passes after.

## 2. Save-view normalization

- [x] 2.1 Change the early-return condition in `save_view` (`crates/pictura-render/src/document_ops/mode/mod.rs`) so the display alpha is trimmed when `doc.source_mode.is_some()` or `doc.channels` is non-empty; update the function comment and the `ponytail:` comment to name the extra-channel path and its ceiling; copy the three color planes with `Plane::read_into` into the new buffer instead of `as_ref()[..].to_vec()`, so a tiled composite is not materialized. Verify the four tests from group 1 pass.
- [x] 2.2 Run `cargo fmt --all -- --check`, `cargo clippy -p pictura-render --all-targets -- -D warnings`, and `cargo nextest run -p pictura-render`; verify all clean.

## 3. Application save paths

- [x] 3.1 Run `cargo nextest run -p pictura_app`, then run the PDF Qt suite `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_save_pdf_' --output-on-failure -j 4` (after a CMake build), and verify the native save, crash-recovery snapshot tests, and PDF export still pass with the trimmed composite.
- [x] 3.2 Run `cargo nextest run -p pictura-codec` and verify the writer suite is unaffected (no codec change is expected).

## 4. Finish

- [x] 4.1 Run `openspec validate --all --strict`, then `openspec archive psd-duplicate-alpha-save --yes`; verify the delta merged into `openspec/specs/color/image-mode-conversion/spec.md` and no non-archived change remains under `openspec/changes/`.
- [x] 4.2 Run `bash scripts/verify-fast.sh`; verify fmt, clippy, test-report, file-size, guard, archive, test-plan, and openspec gates pass.
