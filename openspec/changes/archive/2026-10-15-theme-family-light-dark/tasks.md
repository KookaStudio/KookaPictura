# Tasks

## 1. Family axis and light ramp

- [x] 1.1 `ThemeFamily` + the explicit light base ramp; family-aware `rampFor`
  (per-family hover/shade direction); `tokens(family, level)`.
- [x] 1.2 Family-aware `styleSheet`, palette, and `apply`; the level-only
  overloads delegate to the dark family so existing call sites keep compiling.

## 2. Runtime switch, persistence, and chrome

- [x] 2.1 `SessionState.themeFamily` save/restore (missing value loads dark).
- [x] 2.2 Preferences General "Theme" selector wired to `Theme::apply` alongside
  Brightness; the frame applies the family on launch.
- [x] 2.3 Menu-bar chrome at the top-right: the theme toggle (sun while dark,
  moon while light; clicking switches family) and the Discord / GitHub link
  buttons (`QDesktopServices::openUrl`).
- [x] 2.4 Developer design-system dialog: family selector + family-aware token
  readout, and family restored on close.

## 3. Icons

- [x] 3.1 `theme.sun` / `theme.moon` (lucide-static, ISC) and `link.github` /
  `link.discord` (simple-icons, CC0); listed in `assets/pictura.qrc` with
  `assets/PROVENANCE.md` entries.

## 4. Tests

- [x] 4.1 `tst_design_system` light-family consistency (surfaces lighter than
  text, derived shades distinct, family-name round-trip).
- [x] 4.2 Build and run `ctest -R '^tst_'`, the self-test, and `verify-full.sh`;
  the family persistence and startup icon are covered by `tst_smoke`.

## 5. Deferred follow-ups

- [ ] 5.1 Colour audit: convert the theme greys a light palette exposes to
  tokens, or allow-list them as data.
- [ ] 5.2 `--dump-theme` emits both families; regenerate the guide.
