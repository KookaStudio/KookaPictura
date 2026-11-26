# Tasks: duotone-spec-parse

## 1. Parser

- [x] 1.1 Add `crates/pictura-codec/src/duotone.rs` with `DuotoneSpec`, `DuotoneInk`, `InkColor`, and `parse_duotone` using the offsets in `design.md` (version, plates, 4×10 ink colors, 4×64 Pascal names, 4×28 transfer curves + override, dot gain, 0/1/4/11 overprint colors); reject `<524` bytes and plate counts outside 1–4.
- [x] 1.2 Re-export `parse_duotone`, `DuotoneSpec`, `DuotoneInk`, `InkColor` from `crates/pictura-codec/src/lib.rs`.

## 2. Tests

- [x] 2.1 A synthetic 524-byte buffer with plate count 2 parses to the expected version, inks (color/name/curve/override), dot gain, and one overprint.
- [x] 2.2 Short buffer and plate counts 0 and 5 return `None`.

## 3. Verification

- [x] 3.1 `cargo nextest run -p pictura-codec`; `cargo nextest run --workspace`.
- [x] 3.2 `cargo test -p pictura-codec --test psd` still green.
- [x] 3.3 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-file-size.sh`, `openspec validate duotone-spec-parse --strict`.
