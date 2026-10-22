# Tasks

The workspace run in task 5.1 needs its own target directory: this machine's
worktrees share one `target`, and a sibling worktree's build of
`pictura_paint` was picked up by this one, which fails to compile the app
crate against a foreign `begin_erase`. With a private `CARGO_TARGET_DIR` the
whole workspace builds and tests clean.

## 1. The plan

- [x] 1.1 Write `test-plan.md` in this change folder with one row per scenario in the `compositing/layer-management` delta, and verify `python3 scripts/check-test-plan.py` reports no missing row and that the two tests it names as not yet existing each have a task here (2.2 and 3.1); the check reports "no unknown test" only once those tests exist, which is task 5.2

## 2. The merge oracle

- [x] 2.1 In `crates/pictura-render/tests/merge_oracle.rs`, give the `layer` helper a blend-mode parameter (its two `merged_alpha_pair` call sites pass `BlendMode::Normal`) and add a `merged_pair` helper that merges a base and a source layer and returns the merged node as interleaved RGBA8; verify `cargo nextest run -p pictura-render --test merge_oracle` still passes
- [x] 2.2 Add `merge_down_matches_imagemagick_for_every_supported_mode`: for the `solid` and `ramp` scenes and each of the 19 modes ImageMagick implements with the same formula, diff the merged node against the committed reference at that mode's tolerance (0, except Vivid Light at 1); verify the test passes
- [x] 2.3 Extend the live oracle with the `ramp` scene's `Multiply` composite so the committed reference is checked against a running `magick` as well, keeping the self-skip when `magick` is absent; verify the test passes with `magick` on PATH
- [x] 2.4 Sanity-check the new oracle once: temporarily force the source layer's blend mode to `Normal` in `crates/pictura-render/src/document_ops/layer_ops/merge.rs`, run the new test, verify it fails, and revert the change — forcing the source to `Normal` fails the hand-computed test (`left: [120, 130, 140, 255, ...]` against `right: [19, 31, 44, 255, ...]`), the mode table (`solid/Darken: 64 samples over tolerance 0 (max delta 136)`) and the live oracle (`ramp/Multiply: 192 samples differ (max delta 222)`); the `Normal`-source test still passes, as it must; the change was reverted and `git diff` on `merge.rs` is empty
- [x] 2.5 Verify `python3 scripts/im_compose.py check` still reproduces every committed fixture
- [x] 2.6 Document the merge oracle in `crates/pictura-render/tests/README.md` beside the compositing oracle: the mode table and its tolerances, the `alpha` scene's `Normal`-only limit, and the hand-computed blend test; verify the README names the new tests and the scenes they use

## 3. The merge contract

- [x] 3.1 In `crates/pictura-render/tests/merge_oracle.rs`, add `merge_down_bakes_the_sources_blend_into_the_pixels`: a source of `(120, 130, 140)` with the `Multiply` blend mode over a target of `(40, 60, 80)` with the `Screen` blend mode yields a merged node whose pixels are `(19, 31, 44)` — `Multiply((40, 60, 80), (120, 130, 140))` — and whose blend mode and opacity are the target's; the target's blend is `Screen` so the assertion cannot pass by accident, since a node that hardcoded `Normal` would fail it; verify the test passes and that the file stays inside the test budget
- [x] 3.2 Replace the `ponytail` comment in `crates/pictura-render/src/document_ops/layer_ops/move_content.rs` with the truth: the cut copy is a `Normal` layer and that merge really does composite it as one, but `merge_down` composites the source with the source's blend, and giving the cut copy the source's blend would apply the blend twice — once against the layer's remaining pixels and once against the backdrop; verify `cargo nextest run -p pictura-render` still passes and that the comment no longer claims `merge_down` composites with `Normal`

## 4. The follow-up

- [x] 4.1 Open a follow-up issue for the residual divergence: the merged node inherits the target's blend mode and opacity, so a target with non-default settings renders the merged result differently from the pre-merge canvas, and the merge composites the pair against a transparent backdrop, so the source's blend sees the target's raw colour; record the issue number in the PR body and in `design.md`
- [x] 4.2 Record the issue's own wrong citation — it points at `merge.rs:77`, which is `merge_scope`, not `merge_down` at `merge.rs:177` — in the comment posted on #795
- [x] 4.3 Record the residual divergence as a row in `docs/dev/doc-code-parity-gaps.md`, linked to the follow-up issue from 4.1, and update the register's row count; verify `TASK_ALLOWS_DOCS=1 scripts/guard.sh` accepts the docs change
- [x] 4.4 Record the new gap in `ROADMAP.md` under the open Layers items, linked to #813, so the roadmap and the parity register agree
- [x] 4.5 Carry the `TASK-ALLOWS-DOCS` marker on the commit that ships the docs change from 4.3, as guard rule 4 requires

## 5. Integration and acceptance

- [x] 5.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo nextest run --workspace`; verify all three pass with no project warning — all three pass: format clean, clippy clean, and 3590 tests passed with 28 skipped, with the workspace run in a private `CARGO_TARGET_DIR` (see the note above)
- [x] 5.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`; verify both pass, the plan check now reporting no unknown test because 2.2 and 3.1 landed
- [x] 5.3 Run `openspec archive merge-down-blend-mode-oracle`; verify only `archive/` remains under `openspec/changes/` and `scripts/check-openspec-archived.sh` passes
- [x] 5.4 Comment on #795 with the finding — the premise does not hold for the merge path, the source's blend mode is honoured and is now checked against ImageMagick, the residual divergence is tracked in #813 — and correct the issue's own citation, which points at `merge.rs:77` (`merge_scope`) rather than `merge_down` at `merge.rs:177`
- [x] 5.5 Open the pull request with the mode table and the sanity-check result in the body, and close #795
