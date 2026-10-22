# Tasks: type on a path

## 1. Tests first

- [x] 1.1 Add the Rust layout, hit, caret, and clipping tests named in `test-plan.md`.
- [x] 1.2 Add the Qt `tst_type_on_path` cases named in `test-plan.md`.
- [x] 1.3 Add the codec `TextFrameSet` round-trip tests named in `test-plan.md`.

## 2. Engine

- [x] 2.1 Add `TextPath` / `TextPathKind` to `pictura-core::type_tool`; carry `path` on `TypeSpec` and `TypeTool`.
- [x] 2.2 Add arc-length and nearest-point helpers to `pictura-core::path`.
- [x] 2.3 Add `pictura-render::type_path`: on-path layout/rendering, area clipping, caret stops, baselines, overflow, path hit-testing.
- [x] 2.4 Route placement, rendering, mask coverage, caret, baselines, overflow, and layer specs through the path.

## 3. Codec

- [x] 3.1 Read and write the document `Txt2` `ResourceDict.TextFrameSet` entries, preserving other global blocks and keys.
- [x] 3.2 Assign entries to text layers by text-layer order on read.

## 4. App

- [x] 4.1 Extend `TypeSetting` with path fields and parse them in the bridge.
- [x] 4.2 Add `type_path_at` and the overflow-point bridge.
- [x] 4.3 Start, preview, commit, reopen, and mask path type from `tool_type.cpp`; draw the path and its overflow marker.

## 5. Gates

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`.
- [x] 5.2 Build the app and run `tst_type_on_path`.
- [x] 5.3 `openspec validate type-on-path --strict`, then archive the change.
- [x] 5.4 Tick the roadmap item and record the capability in `docs/dev/STATE.md`.
