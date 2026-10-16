# Design

## Context

`Theme::tokens(family, level)` is the single source of the theme contract: it
returns a `QVector<ThemeToken>` of colour tokens and metrics, and
`Theme::styleSheet` substitutes each key into the global QSS via
`qss.replace("${" + key + "}", value)`. `main.cpp --dump-theme` serialises the
list generically into JSON, `scripts/gen-styleguide.py` turns the dump into
`design-system.html`, and custom-painted widgets read single tokens through
`Theme::tokenColor("<key>")`.

The keys are the only surface this change touches. The mechanism (single list,
`${key}` substitution, generic dump) stays exactly as it is. See
`proposal.md - Why` for motivation.

## Goals / Non-Goals

**Goals:**

- Rename every token key to a semantic role name, kebab-case, one-to-one.
- Keep the substitution and `Theme::tokens` the single source; no structural
  change to the token layer.
- Update every literal-key consumer in the tree so the rename is complete and
  mechanical.
- Archive the full mapping here so a reviewer can verify it is one-to-one.

**Non-Goals:**

- No value, colour, count, family, or level change.
- No new token, no removed token, no split/merge of a role.
- No rename of C++ identifiers that are not token keys: the `Theme::k*`
  metric constants, `Theme::panelColor()` / `workspaceColor()` / `tokenColor()`,
  `Theme::shade()`, and Qt's own enum names (`QPalette::Window`,
  `QPalette::WindowText`, …). They are not tokens and renaming them ripples to
  callers for no contract benefit.
- No change to the dump JSON structure or the guide's markup — only the key
  strings inside them.

## Decisions

### D1 — kebab-case key names

Keys become `surface-recessed`, `content-on-selection`, `border-input`. Rationale:
the intended future is CSS-variable-driven custom themes, where
`--surface-recessed` is idiomatic; the generated guide already emits CSS custom
properties from the dump, so kebab-case makes the dump a valid variable source
directly. Alternative considered: keep the existing camelCase
(`surfacePanel`) for a smaller diff — rejected because it keeps two conventions
(camelCase tokens, kebab CSS vars) that this change is meant to unify.

### D2 — all three token groups renamed

Colour tokens, numeric metrics, and the three pseudo-tokens are all part of the
contract, so all three rename together. Leaving metrics camelCase would keep
the inconsistency the change exists to remove.

### D3 — the `Ramp` struct fields rename to match

`Ramp` (`window`, `panel`, `inputBg`, …) backs the colour token values. Renaming
only the string keys would leave `color("surface-input", ramp.inputBg)` in
`theme.cpp` — confusing and impossible to review against the mapping. The
`Ramp` fields take the same names as their tokens. This is cosmetic: the two
aggregate initialisers (`kLevel1Ramp`, `kLight1Ramp`) are positional, so only
the struct definition and the `ramp.<field>` accesses change — in `rampFor`,
`paletteFor`, `Theme::tokens`, `workspaceColor` (both overloads), and
`panelColor`. `Theme::tokenColor` iterates the token list and touches no `Ramp`
field.

### D4 — mechanical, name-only edits

Every change is a key string or a matching field identifier. No call is
reordered, no value edited, no control flow touched. The rename is done from the
mapping tables below (D5, D6), one group at a time, then verified by grep and by
the existing drift guard.

### D5 — colour-token mapping (39)

| old | new | old | new |
| --- | --- | --- | --- |
| `window` | `surface-recessed` | `panel` | `surface` |
| `windowText` | `content` | `activeTab` | `surface-tab-active` |
| `text` | `content-body` | `workspace` | `surface-canvas` |
| `button` | `surface-control` | `panelHeader` | `surface-header` |
| `buttonText` | `content-control` | `hover` | `surface-hover` |
| `highlight` | `selection` | `pressed` | `surface-pressed` |
| `highlightedText` | `content-on-selection` | `iconHover` | `surface-control-hover` |
| `toolTipBase` | `surface-tooltip` | `iconHoverBorder` | `border-control-hover` |
| `toolTipText` | `content-tooltip` | `iconPressed` | `surface-control-active` |
| `link` | `content-link` | `panelHeaderBorder` | `border-header` |
| `disabledText` | `content-disabled` | `scrollbar` | `surface-scrollbar` |
| `accent` | `accent` | `scrollbarThumb` | `scrollbar-thumb` |
| `noticeBg` | `surface-notice` | `scrollbarThumbHover` | `scrollbar-thumb-hover` |
| `noticeText` | `content-notice` | `graphFill` | `graph-fill` |
| `menuDisabledText` | `content-disabled-menu` | `graphAxis` | `graph-axis` |
| `border` | `border` | `tableBg` | `surface-field` |
| `separator` | `border-subtle` | `alternateTableBg` | `surface-field-alternate` |
| `separatorLight` | `border-subtle-alt` | `inputBg` | `surface-input` |
| `inputBorder` | `border-input` | `buttonBorder` | `border-control` |
| `buttonPressed` | `surface-control-pressed` | | |

Notes on the choices:

- `highlight` is the selection/emphasis fill (menu selection, text selection,
  focus ring); `selection` names that role. `accent` is the separate #2a7fff
  drop/outline accent and keeps its name.
- `panel` is the dominant application surface — `QMainWindow`, menus, toolbars,
  panels, status bar, all painted `${panel}` — so it takes the generic
  `surface`. `window` is the *darker* strip behind dock titles and inactive
  document tabs, so it becomes `surface-recessed`; `workspace` (the empty
  canvas) becomes `surface-canvas`. The three are no longer ambiguous.
- `content-disabled` and `content-disabled-menu` are the two disabled-text
  roles; the menu one is the derived shade. The issue's illustrative
  `content-muted` / `border-strong` have no current 1:1 key, so they are not
  introduced — a pure rename cannot add roles.
- `separator` / `separatorLight` are the two line weights → `border-subtle` /
  `border-subtle-alt`.

### D6 — metric and pseudo-token mapping (8 + 3)

| old | new |
| --- | --- |
| `panelBorderWidth` | `border-width-panel` |
| `chromeBorderWidth` | `border-width-chrome` |
| `groupDividerWidth` | `divider-width-group` |
| `slotBaseW` | `slot-base-width` |
| `slotBaseH` | `slot-base-height` |
| `slotIconMaxW` | `slot-icon-max-width` |
| `slotIconMaxH` | `slot-icon-max-height` |
| `shadeStep` | `shade-step` |

| old pseudo-token | new |
| --- | --- |
| `${tabFontSize}` | `${font-size-tab}` |
| `${inputFontSize}` | `${font-size-input}` |
| `${selectChevron}` | `${select-chevron}` |

### D7 — consumer edits are scoped, not blind sed

A repository-wide rename would corrupt places where the same word is not a
token key:

- `design_system_dialog_pages.cpp` uses `QStringLiteral("hover")` /
  `"pressed"` both as display labels (story/state captions) and as token keys
  (foundation groups, page key lists). Only the token-key uses change.
- The same file's `note()` prose quotes token names (`menuDisabledText`,
  `inputBg`, `inputBorder`, `panelHeader`, `buttonBorder`, `iconHover`,
  `separator`, `tableBg`, `toolTipBase`, `highlight`, `workspace`, …). These are
  documentation and must be updated too, or the in-app browser advertises tokens
  that no longer exist. They are prose, not identifiers, so this part is
  hand-edited.
- `tst_keyboard_shortcuts.cpp:363` asserts a dialog's info text contains the
  word `"workspace"` — a documentation word, not a token.
- QSS selectors (`QMainWindow`, `QTabBar::tab:hover`) and Qt enums
  (`QPalette::Window`, `QPalette::WindowText`, `QPalette::ToolTipBase`) contain
  the old words but are not token keys.

Edits are therefore made per file against the object/key context, and verified
by grepping for `${oldkey}` and `"oldkey"` (quoted), not the bare word.

### D8 — no spec delta

Pure rename, identical values and behaviour, so no requirement changes;
`.openspec.yaml` sets `skip_specs: true` (see proposal.md - Capabilities).

## Risks / Trade-offs

- **[A `${key}` prefix collision after kebab-casing]** — `${surface}` is a
  prefix of `${surface-hover}` → Mitigation: substitution matches the closing
  `}` (`"${surface}"` is not a substring of `"${surface-hover}"`), so no
  collision; `stylesheetSubstitutesEveryToken` already asserts no `${` survives.
- **[A missed literal-key consumer]** — a stale `tokenColor("inputBg")` would
  silently return an invalid colour → Mitigation: grep every old key as a
  quoted literal and as `${…}`; `paintedWidgetsUseTokens` and the browser test
  fail on an invalid token colour.
- **[The guide drifts from the dump]** — `gen-styleguide.py` hard-codes
  `var(--old)` in its CSS → Mitigation: update the `STYLE` constant and
  regenerate; a mismatch is a compile-time no-op but a visual regression the
  idempotency re-run surfaces as a diff to review.
- **[Stale prose in the in-app browser]** — `design_system_dialog_pages.cpp`
  `note()` strings quote the old token names, so the browser would document a
  contract that no longer exists → Mitigation: task 2.2 updates the prose
  alongside the key literals; a grep in task 4.2 catches a missed name.
- **[Diff noise hides a real change]** — a 50-key rename is a large diff →
  Mitigation: the mapping tables (D5/D6) plus the value-unchanged assertion in
  `tst_design_system` (`tokenValue == Theme::k*`, exact colour compares) prove
  only names moved.

## Migration Plan

Single atomic change on the `refactor/419-semantic-theme-tokens` branch; no
runtime migration and no persisted data touches the keys (the session store
persists the family and level by value, not by token key). Rollback is
`git revert`. `design-system.html` is regenerated in the same commit.

## Open Questions

None.
