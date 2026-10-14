# Design

## Context

See `proposal.md` for motivation. The token layer is
`crates/pictura-app/cpp/theme.{h,cpp}`: `Theme::tokens(int level)` returns the
`ThemeToken` list, `styleSheetFor` substitutes it into one global QSS, and
`main.cpp` serialises the same list as `--dump-theme` JSON. Four custom-painted
widgets still hard-code theme greys: `panels/curve_widget.cpp` (plot fill,
histogram fill, grid, baseline, curve, points, border),
`panels/spectrum_bar.cpp` (a single border pen),
`panels/histogram_panel.cpp` (plot fill and frame), and
`panels/navigator_panel.cpp` (thumbnail fill and placeholder text).
`scripts/guard.sh` holds nine non-goal checks (Adobe assets, milestone names,
docs marker, legal artifacts, release markers); none concerns widget colours, so
this change introduces the first colour guard rather than mirroring an existing
one.

## Goals / Non-Goals

**Goals:**

- Five remaining controls tokenised in the one global stylesheet, states per the
  #249 contract.
- The four custom-painted widgets derive theme colours from tokens.

**Non-Goals:**

- Any light family, density preset, or token-dump change (those are separate
  changes).
- Reworking the guide's content or layout (the styleguide change does that).
- Re-tokenising controls already covered in #249.
- `QTextEdit`: the app never instantiates it (only a `qobject_cast` guard in
  `text_input.h`), and the in-use multi-line edit is the already-tokenised
  `QPlainTextEdit`.

## Decisions

**D1 — Which controls, and scoping.** Add rules for `QGroupBox`,
`QRadioButton`, `QProgressBar`, `QHeaderView`, and a general `QSplitter`.
`QGroupBox::title` takes the bold treatment (the same token-level style the
`#dialogTitle` label expresses); group boxes otherwise take `separator`/`text`.
`QRadioButton` reuses the `QCheckBox::indicator` token set (round via
`border-radius`). `QProgressBar` uses `inputBg`/`inputBorder`/`highlight`/`text`.
`QHeaderView::section` uses `panelHeader`/`separator`/`text`. `QSplitter::handle`
generalises the existing `#panelColumnSplitter` rule. Object names only where a
surface must differ from the default.

**D2 — Custom-painted tokens.** Map to existing tokens where the value already
has a role and add only what has none:

| Paint site | Token |
|---|---|
| curve plot background, spectrum interior, histogram fill, navigator thumbnail | `inputBg` |
| curve grid lines, edges, spectrum border, histogram frame | `inputBorder` |
| curve stroke, selected point fill, navigator placeholder text | `text` |
| curve unselected point fill | `inputBg` |
| curve histogram fill, grid sub-line | new `graphFill` |
| curve baseline | new `graphAxis` |

Two new colour tokens (`graphFill`, `graphAxis`); everything else reuses an
existing role. `graphFill` and `graphAxis` are paint-only — the stylesheet never
references them — so this change also relaxes the token-contract scenario
`ds_tokens_single_source` to exempt paint-only tokens from the
stylesheet-substitution check (and `tst_design_system`'s
`stylesheetSubstitutesEveryToken` skips them). The curve's black→white ramp
gradients stay as greyscale data (allow-listed).
`CurveWidget`/`SpectrumBar`/histogram/navigator still own their `paintEvent`;
only the colour source changes. Alternative considered: a whole `graph*` token
family — rejected as duplication of `inputBg`/`text`/`inputBorder`.

**D3 — Literal guard.** A small `scripts/check-colour-literals.py` scans
`crates/pictura-app/cpp/**/*.{cpp,h}` for theme-like **grey** literals — a
`QColor(r, g, b)`/`QColor(0xNN, …)` whose channels are equal, and grey
`Qt::<named>` values — excluding `theme.cpp` and a baseline
`scripts/colour-literal-allowlist.txt`. The allow-list is seeded from the
tree's current grey-literal hits so the check passes on landing: the genuine
data-colour sites (canvas backdrop, marching-ants pairs, greyscale gradients,
the checkerboard, the foreground/background tool swatches, liquify/gradient
preset colours) and the theme-like literals outside this change's widget scope
that are still pending conversion (for example `lens_flare_dialog.cpp`,
`levels_dialog.cpp`), each with a reason. The list only shrinks as the
light-theme change converts them. Narrowing to equal-channel greys avoids
flagging the many legitimate chromatic data literals. It runs from
`scripts/guard.sh` so `verify-fast` and `verify-full` pick it up; CI already runs
`guard.sh`. Alternative considered: fold the scan into `guard.sh`'s existing
greps — rejected because the allow-list and reporting are clearer as a
dedicated, testable script.

**D4 — Test additions.** `tst_design_system` gains the five selectors in
`stylesheetDeclaresPrimitives` and a paint assertion for each converted widget
(grab the widget and compare a sample pixel to the token).

## Risks / Trade-offs

- [A general `QSplitter` rule could restyle unrelated splitters] → keep it to
  `handle` colour/width only; the panel-column rule stays object-scoped and
  wins by specificity.
- [The allow-list can rot] → it starts as a documented baseline and may only
  shrink as sites are converted; the guard names new offenders so drift is
  visible in review.
- [An equal-channel filter misses a tinted theme literal] → acceptable: the
  common theme drift is greys on the grey ramp; a chromatic theme literal is a
  review concern.
- [Palette/token mismatch for a painted widget] → the paint assertion compares a
  rendered pixel to the token value, so a wrong mapping fails the build.

## Migration Plan

Land as one change: tokens and primitive QSS first (stylesheet behaviour
extends, nothing removed), then the widget conversions, then the guard. No
persisted state or data-format change; rollback is a revert.

## Open Questions

None.
