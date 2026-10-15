# Design

## Context

See `proposal.md` for motivation. `theme.cpp` holds one explicit dark base
palette (`kLevel1Ramp`) and derives three further levels by shifting every
surface with `Theme::shade(color, level - kDefaultLevel)`. `Theme::tokens(int)`
feeds both `styleSheetFor` and the `--dump-theme` JSON; `Theme::apply(int)` sets
Fusion, the palette, and the global stylesheet. The brightness preference lives
in `SessionState.brightnessLevel`, is edited in `preferences_dialog.cpp`, and is
applied through `frame.cpp`'s `applyBrightness`. `SvgIconEngine` recolours by the
palette foreground and keys on `paletteGeneration()`, so icons follow a palette
swap for free. The `--dump-theme` JSON keeps its `levels` array (the dark
family) and gains a `families` array carrying both families, so the dump stays a
complete read of the contract.

## Goals / Non-Goals

**Goals:**

- A `dark` and a `light` family with identical role keys, both four levels.
- The family chosen at runtime, persisted, and restored.
- Every theme colour resolves correctly under either family.

**Non-Goals:**

- Changing the dark palette values or the token keys.
- A high-contrast or custom-accent family (a follow-up if wanted).
- Density (separate change).

## Decisions

**D1 — Family axis.** Add `enum class ThemeFamily { Dark, Light }`,
`kFamilyCount = 2`, `kDefaultFamily = Dark`. `Theme::tokens(ThemeFamily, int)`
replaces `tokens(int)`, with the existing call sites updated; a default argument
keeps the diff small. `Theme::apply(ThemeFamily, int)` and
`Theme::styleSheet(ThemeFamily, int)` mirror it. `rampFor(family, level)` picks
the family's base ramp and applies the same shade step, clamped per family's
level range (both four).

**D2 — Light base ramp.** An explicit `kLight1Ramp`, mirroring `kLevel1Ramp`
role-for-role. Initial values (contrast-tuned during implementation):

| Role | Dark (level 1) | Light (level 1) |
|---|---|---|
| windowText / text / buttonText | `#e0e0e0` | `#2b2b2b` |
| disabledText | `#686868` | `#9a9a9a` |
| toolTipBase / toolTipText | `#3a3a3a` / `#e8e8e8` | `#f0f0f0` / `#2b2b2b` |
| link | `#6fa8dc` | `#1a5fa8` |
| window | `#363636` | `#d9d9d9` |
| panel / activeTab | `#4d4d4d` | `#e4e4e4` / `#f0f0f0` |
| workspace | `#1f1f1f` | `#f0f0f0` |
| panelHeader | `#363636` | `#d0d0d0` |
| separator | `#404040` | `#c0c0c0` |
| tableBg | `#404040` | `#ffffff` |
| border | `#2e2e2e` | `#b0b0b0` |
| inputBg / inputBorder | `#3b3b3b` / `#595959` | `#ffffff` / `#a0a0a0` |
| button / buttonBorder | `#3b3b3b` / `#595959` | `#ededed` / `#a0a0a0` |
| buttonPressed / iconPressed | `#303030` / `#2e2e2e` | `#d0d0d0` |
| iconHoverBorder | `#595959` | `#a0a0a0` |
| highlight / highlightedText | `#3d6f99` / `#ffffff` | `#3d6f99` / `#ffffff` |
| accent | `#2a7fff` | `#2a7fff` |

**D3 — Shade direction per family.** Dark derives `hover = shade(panel, +1)`
(lighter); light derives `hover = shade(panel, -1)` (darker), since hover on a
light surface reads as a darkening. The same direction applies to every derived
shade: `rampFor` takes a family direction and passes it to the derived tokens.
This matters most for `alternateTableBg`: the dark family derives
`shade(tableBg, +1)`, but with a light `tableBg` of `#ffffff` a `+1` shade
clamps back to white and alternate rows vanish; the light family derives
`shade(tableBg, -1)`. `menuDisabledText`, `separatorLight`, `panelHeaderBorder`,
and `scrollbarThumbHover` follow the same rule. Alternative considered: derive
light shades by the dark direction and hand-pick light base values — rejected
because it silently couples every derived token to a hand-tuned base.

**D4 — Runtime switch and persistence.** Add `SessionState.themeFamily` (0 dark
/ 1 light), default dark, persisted in `session.cpp`. Add a family selector to
the Preferences General page next to Brightness, emitting a
`themeFamilyChanged` signal the frame connects to a `applyTheme(family, level)`
that calls `Theme::apply` and repaints. A session with no value loads dark.
Shortcuts are optional and out of scope.

**D5 — Colour audit (follow-up).** The literal guard from
`design-system-remaining-controls` names the remaining theme greys a light
palette exposes (for example `lens_flare_dialog.cpp`'s `#2b2b2b`/`#141414`
surface and border, `blur_gallery_light_range.h`'s grid lines). Each is converted
to a token or added to the guard's allow-list as documented data. `image_view`
canvas backdrop, selection marching-ants, and preset palettes stay data (allow-
listed). `SvgIconEngine` needs no change. Not yet done (task 5.1).

**D6 — Tests.** `tst_design_system` loops both families in
`paletteMatchesTokens` and the token-shape checks, and asserts the light surface
tokens are lighter than their text/icon roles and derived shades stay distinct.
`tst_smoke` covers family persistence and the restored startup icon. Every other
call site of the changed signatures is updated: `tst_retouch_tools.cpp:324`,
`tst_command_tree.cpp:381`, `selftest.cpp:1638,1643` (stylesheet) and
`tst_icon_assets.cpp:132,136,138` (apply).

## Risks / Trade-offs

- [A light palette exposes hard-coded greys across many dialogs] → the audit is
  part of the change and the literal guard makes remaining offenders visible.
- [Deriving light levels by `shade()` can wash light surfaces toward white] →
  tune the light base and step; the per-family clamp prevents range overflow.
- [A light theme can fail contrast where dark passed] → assert the
  surface-lighter-than-text relation and tune the initial values; a formal
  contrast check is a follow-up if wanted.
- [Missing a call site of `tokens(int)`] → the compiler flags the signature
  change; a default argument, if used, is removed before merge so nothing is
  missed.

## Migration Plan

One change: family axis and light ramp first, then the switcher and persistence,
then the dump families, then tests; the colour audit and guide are follow-ups. A
stored session without a family loads dark, so no migration is needed; rollback
is a revert.

## Open Questions

None.
