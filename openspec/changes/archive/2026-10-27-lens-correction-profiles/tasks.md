# Tasks: lens-correction-profiles

## Tests

- [x] Rust: every `pictura-filters`, `pictura-codec`, `pictura-render`, and `pictura-app` test named in `test-plan.md`
- [x] Qt: new `tst_lens_profiles` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`; `tst_lens_correction` slot fixtures at 36 slots

## Engine

- [x] `LensModel`: polynomial distortion `c1…c6`, per-channel CA `[v, c, b]`, `rescaled`
- [x] Map: general radial polynomial; per-channel CA polynomial times the fringe scale
- [x] Profiles: `raw`, `[sub]`, sub-profiles, stable ids, matching for lens-only profiles, Prefer RAW, sensor-size order, sub-profile pick
- [x] `lensfun.rs`: `quick-xml` parser and the exact model conversion
- [x] Slots: 36, the tail layout, `from_slots` with the flag and the legacy id; catalogue row; codec keys and defaults

## App

- [x] `filter_tools/lens_profiles.rs`: search roots, file replacement, user folder, `installed()`
- [x] Bridge: stable ids, sub-profiles, auto pick from EXIF, `lc_profile_slots`, Prefer RAW
- [x] C++: Search Criteria pop-up with Prefer RAW, sub-profile context menu, "Profile unavailable", the 20-slot tail, Search Online tooltip

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/06-filters/lens-correction.md`, `assets/PROVENANCE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
