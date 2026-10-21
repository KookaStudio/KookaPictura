# Tasks

## 1. The roadmap

- [x] 1.1 Re-cut `ROADMAP.md` into an "At a glance" area-to-status table, a "What works today" section whose entries each cite a path in the tree, and a "Not planned" list; verify every shipped tool group from `crates/pictura-app/cpp/tool_catalog.cpp` appears ticked and that the two 3D groups appear under "Not planned"
- [x] 1.2 Write the "Open work" section as thirteen domain sections of feature-cluster bullets; verify every bullet carries at least one `#NNN` issue reference and that every issue named resolves to a row in `docs/dev/doc-code-parity-gaps.md` or to a prose-tracked issue in its §F
- [x] 1.3 Correct the "Not planned" list: drop measurement and counting, which ship as the Count tool; verify `docs/00-overview/feasibility-and-non-goals.md` no longer lists counting as a non-goal
- [x] 1.4 Link `docs/dev/doc-code-parity-gaps.md` from the roadmap as the authoritative gap index; verify the link resolves

## 2. The README

- [x] 2.1 Extend the feature list with the shipped surface: the healing family, pen and path tools, type tools and type masks, shape tools, magnetic lasso, rotate view, slice tools, colour sampler/ruler/note/count, the transform family, Save for Web, printing, and the export formats including GIF, SVG and PDF; verify each new bullet names a path in the tree
- [x] 2.2 Add shape and type layers to the Layers bullet, and magnetic lasso, Refine Edge and Colour Range to the Selections bullet; verify the claims against `crates/pictura-core/src/lib.rs` and `crates/pictura-select/src/`
- [x] 2.3 Replace the two over-claims with the real counts: 110 filters with the five absent CS6 families named, and the adjustment families with the control each one still lacks; verify against `crates/pictura-app/src/cxxqt_object/filter_map/arities.rs` and `crates/pictura-adjust/src/types.rs`
- [x] 2.4 Mention PDF and EPS in the quick start, and link the gap register from the developer section; verify both

## 3. The corpus banner

- [x] 3.1 Replace the "No code exists…" sentence in all 137 corpus files with the true sentence, keeping each file's source-provenance sentence intact; verify the replacement is byte-identical in every file and that no file still carries the claim
- [x] 3.2 Correct `docs/README.md`: the "Documentation-only specification" title, the "no code, no build files, and no dependencies" banner, and the parity-target line; verify the entry point no longer denies the implementation
- [x] 3.3 Correct `docs/00-overview/product-overview.md`: the "is a specification corpus, not a program" paragraph, the "contains no source code" claim, and acceptance criterion 4; verify criterion 4 keeps its true half, that the mapping sections are still labelled proposals, and that the parity target matches the shipped Count tool
- [x] 3.4 Link the missing `01-architecture/build-and-packaging.md` references to #494 rather than authoring the file; verify no reference is left dangling

## 4. The register

- [x] 4.1 Correct `FILT-1`: the Lighting Effects engine ships; verify `crates/pictura-filters/src/render/lighting.rs` and `filter_map/arities.rs`
- [x] 4.2 Correct `LAY-7`: Rubylith display options ship; verify `crates/pictura-app/cpp/mask_display_options_dialog.cpp`
- [x] 4.3 Correct §E: linked and embedded objects are parsed by the codec; verify `crates/pictura-codec/src/smart_object.rs`
- [x] 4.4 Correct the two stale §D rows and drop the two this change makes stale; verify each remaining §D row still describes a real doc-code divergence
- [x] 4.5 Note the sweep in the register's front matter so the next reader knows the rows were re-checked on this date; verify the note names the sweep issue

## 5. The guard

- [x] 5.1 Add `scripts/check-doc-claims.py`: strip blockquote markers, normalise whitespace, and fail when a file under `docs/` outside `docs/dev/` asserts the repository contains no code; verify the 26 wrap-hidden banners are caught by running it against the pre-sweep tree
- [x] 5.2 Add the same script's roadmap check: every unchecked `ROADMAP.md` item carries a `#NNN` issue reference; verify it fails on a roadmap item with no issue
- [x] 5.3 Call the script from `scripts/guard.sh` as rules 12 and 13, in the style of the existing rules; verify `bash scripts/guard.sh` passes on the swept tree and fails on the pre-sweep tree

## 6. The process rule

- [x] 6.1 Add `AGENTS.md` rule 12 requiring a pull request that ships or removes a feature to update `ROADMAP.md` in the same change; verify it sits after rule 11 and matches the surrounding style

## 7. The state anchor

- [x] 7.1 Add a newest-first bullet to `docs/dev/STATE.md` "Where things are" recording the sweep, its evidence base, the register corrections, and the new capability; verify it is the first bullet under the heading

## 8. Integration and close-out

- [x] 8.1 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every scenario row is covered, every named test exists, and the change validates
- [x] 8.2 Run `bash scripts/guard.sh` with `TASK_ALLOWS_DOCS=1` and `python3 scripts/check-openspec-archived.sh`; verify both pass
- [x] 8.3 Run `openspec archive documentation-corpus` and re-run `openspec validate --all --strict`; verify no non-archived change remains and that the archived folder keeps its `test-plan.md`
