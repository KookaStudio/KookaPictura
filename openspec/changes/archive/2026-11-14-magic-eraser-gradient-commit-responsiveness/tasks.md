# Tasks

## Tests first

- [x] Add `aMagicEraserCompositesItsRegion` and
      `aStyledMagicEraserRecompositesWhole` for `mgc_magic_region` and
      `mgc_magic_styled`.
- [x] Add `aCancelledMagicEraserJobChangesNothing` for `mgc_magic_job`.
- [x] Add `a_uniform_erase_writes_uniform_alpha_tiles` and
      `a_uniform_erase_matches_the_per_pixel_path` for `mgc_uniform_tiles` and
      `mgc_uniform_parity`.
- [x] Add `layer_from_background_gives_a_uniform_alpha_plane` and
      `ensure_alpha_gives_a_uniform_plane` for `mgc_uniform_alpha`.
- [x] Add `an_opaque_full_strength_normal_fill_writes_the_source` for
      `mgc_opaque_fill`.

## Implementation

- [x] Return the dirty rectangle from `magic_erase_at` as a bounded edit, with
      the styled-document fallback.
- [x] Run the Magic Eraser through `runDocumentCommand` and `document_op` on a
      large armed document; add the post-land hook for Fade.
- [x] Write uniform tiles from `magic_erase` without visiting their samples.
- [x] Make `release_background` and `ensure_alpha` use `Plane::filled`.
- [x] Add the opaque full-strength Normal fast path to `Pass::blend_one`.

## Verification

- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo nextest run --workspace` (all pass except the ImageMagick fixture
      oracle, which fails on this host's ImageMagick 7.1.2-27 and is unrelated
      to this change)
- [x] `ctest --test-dir build -R '^tst_tool_commit$' --output-on-failure`
- [x] `openspec validate --all --strict`
- [x] Run `scripts/profile-tools.py --size 17000x17000 --scenarios
      magic-eraser,gradient` before and after, and record the numbers in the PR.
