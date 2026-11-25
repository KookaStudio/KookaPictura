# Tasks

## 1. Test ownership moves

- [x] 1.1 Move the two source-free content constructor tests, the opaque-text round-trip test, and the admitted-content budget test into a render integration suite; verify all original assertions and fixture paths remain present.
- [x] 1.2 Move the four render-authored vector-mask tests and two render-authored layer-record tests into a render integration suite; verify their psd-tools scripts and expectations remain unchanged.
- [x] 1.3 Move the native-depth conversion test into `native_depth_roundtrip.rs` and the flatten-then-profile-conversion test into `document_oracle.rs`; verify both retain every assertion.

## 2. Codec-local fixtures

- [x] 2.1 Replace render layer and shape setup in `mask_params_oracle.rs` with codec-local `LayerParts` fixtures; verify density, feather, default-flag, real-mask, and psd-tools assertions remain unchanged.
- [x] 2.2 Replace render mask-link calls in `mask_link_state.rs` with direct PSD flag fixtures; verify linked and unlinked PSD/PSB and psd-tools assertions remain unchanged.
- [x] 2.3 Build the streamed raster-save test's tiled composite through `pictura_core::Plane::tile`; verify the save still receives a four-channel tiled buffer and the heap assertions remain unchanged.
- [x] 2.4 Point `layer_content.rs` encoder calls at `pictura_codec`; verify every decode expectation and malformed-input assertion remains unchanged.

## 3. Dependency removal and guard

- [x] 3.1 Remove `pictura-render` from codec dev-dependencies and its codec package entry in `Cargo.lock`; verify no executable codec source or test still calls `pictura_render`.
- [x] 3.2 Extend the existing render feature-gate manifest test to reject a codec render dev-dependency; verify the test count is unchanged and the assertion reads the codec dev-dependency table.

## 4. Review and verification

- [x] 4.1 Compare original and final test inventories and confirm all twelve moved tests remain with their assertions and ignore reasons; record the source-review result in `test-plan.md`.
- [x] 4.2 Review the final diff for fixture ownership, dependency direction, file-size caps, and absence of `pictura-core` mutation changes.

## 5. Verification

- [x] 5.1 `cargo fmt --all`.
- [x] 5.2 Full CMake configure and build under `scripts/with-build-lock.sh`.
- [x] 5.3 `cargo tree -p pictura-codec --edges dev` holds no `pictura-render` or `wgpu`; codec and render test suites pass with the count preserved.
- [x] 5.4 `cargo clippy -p pictura-codec -p pictura-render --all-targets -- -D warnings`.
