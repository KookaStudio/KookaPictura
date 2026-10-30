# Tasks

## 1. Engine

- [x] 1.1 Add `SmartObject::nesting` (default 0, excluded from equality, never written).
- [x] 1.2 Set nesting on every smart object `decode_embedded` reads; refuse `embedded_source` at `MAX_EMBED_NESTING`; hash nesting into the embedded key.
- [x] 1.3 Record the claiming thread in `State::Building`; never wait on a build the caller owns.
- [x] 1.4 Add the four tests from the test plan.

## 2. Gates

- [x] 2.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 2.2 `cargo nextest run -p pictura-core -p pictura-render -p pictura-codec`; prove the new tests fail without the fix.
- [x] 2.3 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`; archive.
