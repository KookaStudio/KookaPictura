# Proposal

## Why

The design-system token keys name implementation surfaces — `window`, `panel`,
`hover`, `border`, `inputBg` — rather than the role a value plays. A theme
holds the key constant and swaps only its value, so the key should read as
intent (`surface`, `surface-hover`, `content`, `border-input`). Role names make
the family and level axes easier to reason about and let future custom themes
target a stable, semantic CSS contract instead of guessing at structure names.

## What Changes

- Rename every theme token key emitted by `Theme::tokens` and substituted into
  the global stylesheet from an implementation-surface name to a semantic role
  name, in kebab-case. This covers all three groups:
  - 39 colour tokens (`panel` → `surface`, `window` → `surface-recessed`,
    `hover` → `surface-hover`, `border` → `border`, `inputBg` →
    `surface-input`, …),
  - 8 numeric metrics (`panelBorderWidth` → `border-width-panel`,
    `slotBaseW` → `slot-base-width`, …),
  - 3 pseudo-tokens handled outside the token list (`${tabFontSize}` →
    `${font-size-tab}`, `${inputFontSize}` → `${font-size-input}`,
    `${selectChevron}` → `${select-chevron}`).
- Rename the backing `Ramp` fields to match, so `theme.cpp` reads coherently
  (`ramp.panel` → `ramp.surface`). Positional aggregate initialisers are
  untouched.
- Update every literal token-key consumer: the six custom-painted widgets that
  call `Theme::tokenColor("<key>")`, the in-app design-system browser
  (`design_system_dialog_pages.cpp`), and the design-system test suites.
- Update the `var(--key)` references in the style-guide generator
  (`scripts/gen-styleguide.py`) and regenerate `design-system.html`.
- Archive the complete one-to-one mapping in `design.md` so reviewers can check
  every key moved exactly once.

**BREAKING**: the token keys in the `--dump-theme` JSON and the CSS custom
properties in `design-system.html` change name. Values, counts, families, and
levels are unchanged. No in-repo consumer relies on the old names after this
change; there is no external consumer to preserve.

## Capabilities

This is a pure refactor: every token value, the token count, both families, the
brightness levels, the stylesheet behaviour, and the dump/guide shape are
identical. Only key names change, so no capability's requirements change.
`.openspec.yaml` sets `skip_specs: true`.

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/cpp/theme.cpp` — token keys, `Ramp` fields, QSS
  placeholders. (`theme.h` holds no token keys and is unchanged.)
- `crates/pictura-app/cpp/design_system_dialog_pages.cpp` — token key literals
  in the browser's foundation groups and story key lists.
- `crates/pictura-app/cpp/panels/{curve_widget,histogram_panel,navigator_panel,spectrum_bar}.cpp`
  and `crates/pictura-app/cpp/acr/{acr_controls,acr_tone_curve_panel}.cpp` —
  `Theme::tokenColor("<key>")` literals.
- `crates/pictura-app/cpp/tests/{tst_design_system,tst_design_system_dialog}.cpp`
  — key literals in the drift guard.
- `scripts/gen-styleguide.py` — the `var(--key)` set in its CSS; then
  `design-system.html` is regenerated.
- No dependency, engine-crate, bridge, or behaviour change. `main.cpp`'s
  `--dump-theme` reads `Theme::tokens` generically and needs no edit.
