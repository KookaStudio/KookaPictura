# Tasks

## 1. Regression tests first

- [x] 1.1 Add the `serialized_profiles_have_a_zeroed_creation_date` unit test to `crates/pictura-color/src/lib.rs`, asserting header bytes 24..36 of a built-in profile's `to_icc()` output are all zero; verify it fails on the current code with `cargo test -p pictura-color serialized_profiles_have_a_zeroed_creation_date`
- [x] 1.2 Add the `serialized_profiles_are_byte_identical` unit test to `crates/pictura-color/src/lib.rs`, asserting two `Profile::adobe_rgb().to_icc()` calls return equal bytes; verify it runs with `cargo test -p pictura-color serialized_profiles_are_byte_identical` (complementary to 1.1, which is the deterministic guard)

## 2. Deterministic serialization

- [x] 2.1 In `Profile::to_icc` (`crates/pictura-color/src/lib.rs`), zero ICC header bytes 24..36 of the serialized bytes **guarded on `bytes.len() >= 36`** so the documented panic-free empty fallback is preserved, with a short comment naming why (Little CMS stamps wall-clock at creation; upstream has no setter); verify `cargo test -p pictura-color` passes, including the new tests
- [x] 2.2 Confirm the existing `icc_round_trips` test still covers scenario `icc_serial_reload` against the zeroed-date bytes, adjusting it only if the round-trip assertion needs the date field; verify `cargo test -p pictura-color icc_round_trips` passes

## 3. Delete the redundant consumer-side masking

- [x] 3.1 Remove the `undated_icc` helper from `crates/pictura-app/src/cxxqt_object/tests.rs` and its four uses plus import in `crates/pictura-app/src/cxxqt_object/tests_impl.rs`, leaving plain byte-equality assertions; verify `cargo test -p pictura-app convert_profile_is_byte_reversible_through_history assign_profile_retags_and_is_reversible_through_history` passes
- [x] 3.2 Remove the manual date-zero loop from `crates/pictura-color/examples/dump_adobe_rgb.rs`, then regenerate and confirm the fixture is unchanged: `cargo run -q -p pictura-color --example dump_adobe_rgb > /tmp/psd_icc_rgb.icc` followed by `cmp /tmp/psd_icc_rgb.icc crates/pictura-codec/tests/fixtures/psd_icc_rgb.icc`

## 4. Integration verification

- [ ] 4.1 Verify the residual date-flaky `pictura-codec` byte comparisons in `crates/pictura-codec/src/icc.rs` and `crates/pictura-codec/tests/profile_assignment_oracle.rs` pass unchanged: `cargo test -p pictura-codec` and `cargo test -p pictura-codec --test profile_assignment_oracle` (PIL oracle; self-skips without PIL)
- [ ] 4.2 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, tests, file-size, guard, and openspec all pass
