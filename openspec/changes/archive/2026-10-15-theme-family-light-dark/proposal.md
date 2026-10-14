# Proposal

## Why

`Theme` ships one dark palette in four brightness levels. The token layer, the
dump, and the persisted preference all assume a single linear ramp, so there is
no light theme and no way to prove the contract holds across genuinely
different palettes. A light palette is not a shade step of the dark ramp, so it
needs a family dimension rather than more levels.

## What Changes

- **Theme family dimension.** Introduce a family axis with a `dark` and a
  `light` family. Role keys are unchanged across families; only the values
  differ — light surfaces (window, panel, workspace, headers, inputs) and dark
  text and icons, mirroring the dark palette role-for-role.
- **Light family in the token layer.** The token accessor, the stylesheet
  builder, and the palette all take a family and a brightness level. The light
  family has its own base palette with four brightness levels, derived the same
  way as the dark family.
- **Runtime switch and persistence.** A family preference is added to the
  session state and the Preferences dialog, applied at runtime through the same
  path as brightness, and restored on launch. Icons recolour through the
  existing palette-following icon engine.
- **Colour audit (follow-up).** Every theme-concern colour that a light palette
  exposes as wrong (remaining hard-coded greys outside the token layer) is to be
  converted to a token or added to the literal-guard allow-list as documented
  data. Not yet done; tracked in the tasks as a follow-up.
- **Dump and guide (follow-up).** `--dump-theme` currently emits the dark
  family's four levels; emitting both families, and rendering the light family
  in the guide through its family switcher, is a follow-up.

The dark palette values, the token keys, and the level count semantics are
unchanged. No breaking change to persisted data: an older session without a
family loads the dark default.

## Capabilities

### New Capabilities

<!-- None: this change extends the existing capability. -->

### Modified Capabilities

- `ui/design-system`: the token-contract requirement is keyed by family and
  level, and a new requirement defines the light family and its persisted
  runtime switch.

## Impact

- `crates/pictura-app/cpp/theme.{h,cpp}` — `ThemeFamily`, the light base ramp,
  family-aware accessor/stylesheet/palette and derived shades.
- `crates/pictura-app/cpp/session.{h,cpp}`, `frame*.cpp`,
  `preferences_dialog.{h,cpp}` — family preference and switcher.
- `crates/pictura-app/cpp/frame_menus.cpp` — the top-right menu-bar chrome: the
  theme toggle and the Discord / GitHub link buttons.
- New `assets/icons/{theme.sun,theme.moon,link.github,link.discord}.svg`
  (`assets/pictura.qrc` + `assets/PROVENANCE.md`).
- Call sites of the changed signatures: `tests/tst_design_system.cpp`,
  `tests/tst_retouch_tools.cpp`, `tests/tst_command_tree.cpp`,
  `tests/tst_icon_assets.cpp`, `selftest.cpp` (stylesheet/apply checks).
- `crates/pictura-app/cpp/**` — theme-grey conversions found by the audit.
- No new dependency, no engine-crate or cxx-qt bridge change.
