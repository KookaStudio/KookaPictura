# Tasks

## 1. Dependency setup

- [x] 1.1 Add `jpeg2k = "0.11"` to `crates/pictura-codec/Cargo.toml` and verify `cargo check -p pictura-codec` succeeds; the commit body states why the pure-Rust port replaces the native OpenJPEG bindings.
- [x] 1.2 Regenerate `flatpak/cargo-sources.json` and `THIRD-PARTY-LICENSES` for the new `jpeg2k`/`openjp2` dependency graph and verify `check-flatpak-cargo-sources.py` passes.

## 2. JPEG 2000 compression

- [x] 2.1 Write `pictura-codec::pdf_jpeg2000_writes_codestream`, `pdf_jpeg2000_quality_changes_size`, and `pdf_jpeg2000_gated_to_zip`, add `tst_save_pdf::jpeg2000IsEnabled`, and extend `tst_save_pdf::jpeg2000IsDisabled` with the Acrobat 5, 16-bit-document-with-Convert-off, and PDF/A-1b cases; verify each fails for the expected reason before implementation.
- [x] 2.2 Implement `crates/pictura-codec/src/pdf/jpx.rs` (CoreImage from interleaved 8-bit planes, `EncodeParameters::tile_size`/`layer_rate`, codestream output) and `Raster::jpx_capable`; verify `cargo test -p pictura-codec pdf_jpeg2000` passes.
- [x] 2.3 Wire `Compression::Jpeg2000` to `/JPXDecode` in `pdf/mod.rs::image_stream` and remove it from the DCT arm; verify `pdfimages -list` on a written test file reports the jpx filter.
- [x] 2.4 Apply the level/depth/PDF-A gating and the Tile Size unlock in `pdf/options.rs`, add the JPEG 2000 Summary line, and keep 16-bit at ZIP; verify `cargo test -p pictura-codec pdf::options` passes.
- [x] 2.5 Add the `documentIs16Bit` constructor context flag and enable the JPEG 2000 item only when the level, the document depth (or Convert 16 Bit/Channel Image To 8 Bit/Channel on), and the standard allow it in `pdf_save_dialog.cpp`; verify `tst_save_pdf::jpeg2000IsEnabled` and `jpeg2000IsDisabled` pass.
- [x] 2.6 Update the JPEG 2000 dependency note in `docs/01-architecture/file-formats.md` to the pure-Rust 0.11 port and verify it matches `Cargo.toml`; commit with `TASK-ALLOWS-DOCS`.

## 3. Spot Colors

- [x] 3.1 Write `pictura-codec::display_info_round_trips`, `spot_channel_indices_read_display_info`, `pdf_spot_payload_keeps_spot_and_alpha`, `pdf_spot_payload_drops_spot_only`, `pdf_spot_payload_drops_alpha_only`, the psd-tools oracle `pictura-codec::pdf_spot_payload_matches_psd_tools`, and `tst_save_pdf::spotColorsFollowTheDocument`; verify each fails before implementation.
- [x] 3.2 Parse and encode PSD image resource 1077 (13-byte entries, mode 2 = spot) and expose `spot_channel_indices`; verify the resource tests pass.
- [x] 3.3 Partition the embedded payload by channel kind and rewrite resources 1045/1006 and 1077 to the retained channels; verify the three payload tests and the psd-tools oracle pass.
- [x] 3.4 Write and parse `PicturaSpotColors` in the non-preset joboptions text, unlock the field, add it to the dialog's kept fields, and add the Summary line; verify `cargo test -p pictura-codec` passes.
- [x] 3.5 Bind the Spot Colors checkbox, disable it when the document has no spot channel, add the `pdf_spot_channel_count` bridge, and pass the flag from `import_save_pdf.cpp`; verify `tst_save_pdf::spotColorsFollowTheDocument` passes.
- [x] 3.6 Update the WF-2 parity row for the spot-channel gap and verify the row no longer claims Spot Colors missing; commit with `TASK-ALLOWS-DOCS`.

## 4. Proof preview engine

- [x] 4.1 Write `pictura-color::proof_preview_changes_colors`, `proof_simulations_differ`, `proof_alpha_untouched`, and `proof_invalid_profile_errors`; verify each fails before implementation.
- [x] 4.2 Implement `crates/pictura-color/src/proof.rs` with the lcms2 proofing transform over interleaved RGBA8 pixels (RGB transformed, alpha copied unchanged), the simulation mapping, and the alpha-preserving wrapper the app calls after unpremultiplying; verify `cargo test -p pictura-color proof` passes.
- [x] 4.3 Export the proof API from `pictura-color` and document the inferred simulation mapping with a `ponytail:` comment naming the upgrade path; verify `cargo clippy -p pictura-color --all-targets -- -D warnings` passes.

## 5. View proof setup and canvas

- [x] 5.1 Add the proof state to `PictureViewRust` and the `proofing.rs` bridge (set setup, set Proof Colors, read name/active); verify `cargo check -p pictura-app` succeeds.
- [x] 5.2 Write the `tst_proof_setup` cases named in the test plan and register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the suite compiles and each case fails for the expected reason.
- [x] 5.3 Apply the proof transform only on the canvas composite branch in `impl_core/display.rs`, leaving the Quick Mask and solo-mask early returns unproofed; verify `tst_proof_setup::canvasShowsProof`, `proofingIsViewOnly`, and `proofSkipsMaskViews` pass.
- [x] 5.4 Implement `frame_proof.cpp` handlers, the Custom dialog, checked providers, the session keys, the apply hook, and the title suffix; add both sources to the CMake fragments; verify the remaining `tst_proof_setup` cases pass.
- [x] 5.5 Mark the four proof catalogue rows implemented, make Working CMYK, Working Gray, and Proof Colors checkable and leave Custom implemented without a checkmark, add their const names, regenerate `command_ids.h`, update `catalogue.tsv`, and verify `scripts/gen-command-ids.py --check` and `cargo test -p pictura-script` pass.
- [x] 5.6 Update the UI-7 parity row for Proof Setup/Colors and verify it no longer lists them as missing; commit with `TASK-ALLOWS-DOCS`.
- [x] 5.7 Update the proof-setup bullet in `docs/01-architecture/color-management.md` to the application-level model and note that the Preserve Numbers and Monitor RGB entries are not modelled; verify the bullet matches the proof-setup delta; commit with `TASK-ALLOWS-DOCS`.

## 6. Use Proof Setup in the PDF writer
- [x] 6.1 Write `pictura-codec::pdf_use_proof_setup_converts_to_device`, `pdf_use_proof_setup_ignores_profile_policy`, `pdf_use_proof_setup_writes_eight_bit`, `pdf_output_conversion_overrides_proof`, `pdf_use_proof_off_keeps_document_tag`, `pictura-render::pdf_live_text_colour_follows_proof`, `tst_save_pdf::proofSetupLocksUnderOutputConversion`, and `tst_save_pdf::proofSetupLabelFollowsTheSetup`; verify each fails before implementation.
- [x] 6.2 Add the `useProofSetup` save option and joboptions key, the locked rule, the 8-bit forcing, the Summary lines, and the `PdfEnv` proof request, and apply it in `build_raster`/`text_fill`, including tagging under Don't Include Profiles; verify the codec and render tests pass.
- [x] 6.3 Bind the checkbox with the live proof name, pass the setup from `import_save_pdf.cpp`, and verify `tst_save_pdf::proofSetupLabelFollowsTheSetup` and the rest of `tst_save_pdf` pass.
- [x] 6.4 Verify `scripts/check-test-plan.py` maps every scenario in both deltas to a row.

## 7. Integration and archive

- [x] 7.1 Tick the PDF writer options item in `ROADMAP.md` and record the milestone in `docs/dev/STATE.md`; commit with `TASK-ALLOWS-DOCS`.
- [x] 7.2 CI runs the workspace fmt, clippy, nextest, and verify-fast gates; local execution was superseded by the user's instruction to let CI verify.
- [ ] 7.3 Run `openspec validate --all --strict`, archive `pdf-writer-jpx-spot-proof`, and verify `bash scripts/check-openspec-archived.sh` passes.
- [ ] 7.4 Confirm the commit sequence: `feat(io)`/`feat(color)`/`feat(ui)` commits for the features and a separate `docs:` commit, every subject ending `(#626)`.
