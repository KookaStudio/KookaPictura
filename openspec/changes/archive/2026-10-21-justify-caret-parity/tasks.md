# Tasks

## 1. The caret's parity with the renderer

- [x] 1.1 Spread the caret's lines the lines the renderer spreads: a wrapped line of a justified paragraph, and the paragraph's last line under Justify All. The condition was inverted, so the caret spread the paragraph's last line and left the wrapped lines unspread.
- [x] 1.2 Follow the renderer's shift rule in the caret: a spread line fills the column, a line that could not be spread stays flush left, and every other line takes the alignment.
- [x] 1.3 Add `pictura-render::the_caret_stands_where_the_spread_glyphs_are_on_a_wrapped_line`: the caret's stops on a spread wrapped line stand at the painted glyphs' pens. Verify it fails with the inverted condition.

## 2. The space-count guard

- [x] 2.1 Report no extra when the shaped run's space glyphs and the line's space characters disagree, so the line is not spread and the advance cannot overshoot the column.
- [x] 2.2 Shape the trimmed line in the caret, so it counts the same space glyphs the renderer widened.

## 3. The module

- [x] 3.1 Move the spread loop, the advance estimate and the flush-left correction into `type_justify`, out of `text_render.rs`, which was over the 800-line target.

## 4. The tests

- [x] 4.1 Give the point-type layer's test two lines of different lengths, so it can detect a per-line alignment regression.
- [x] 4.2 Assert the paragraph's last line with no word space takes its variant's alignment, not the flush left of a line that could not be spread.

## 5. The corpus

- [x] 5.1 Scope the spread requirement to horizontal type, state the vertical paragraph's placement, and state the wrap's end-boundary convention in the caret scenario.
- [x] 5.2 Say that a justified paragraph's conversion to point text loses the spread, so the requirement's "keeping the first line where it was" does not contradict it.
- [x] 5.3 Say which justification bytes the Qt `Rasterize Type` path honours.

## 6. The record and the delivery

- [x] 6.1 Add the no-word-space rule and the point-type alignment to the `docs/dev/STATE.md` entry's ceilings.
- [x] 6.2 Correct the archived change's task 1.6, which claimed its test failed before the fix when it guards the new rule.
- [x] 6.3 Run `openspec validate --all --strict`, then `openspec archive` the change and push the pull request.
