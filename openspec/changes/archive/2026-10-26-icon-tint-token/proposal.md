# Proposal

## Why

Bundled SVG icons are tinted with the shared `QPalette::ButtonText` control-text
colour (`#E0E0E0`) rather than an icon-specific value. Icons therefore read as
slightly dim, and the design system has no way to tune glyphs without also
changing button text (#315).

The 80 % render opacity already in `SvgIconEngine::render` stays. Only the base
colour moves to a dedicated token: `#FFFFFF` on the dark family so glyphs read
crisp against the chrome, and the light family's dark content colour so they
stay legible on light surfaces.

## What Changes

- Add a dedicated `icon` token to the theme ramp and token contract.
- Render SVG icons with that token instead of `QPalette::ButtonText`.
- Keep disabled icons on the palette's disabled colour.
- Keep the 80 % opacity and the explicit `icon(id, color)` override.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/icon-assets`: the active-theme requirement names the dedicated icon tint
  token and its per-family value instead of the palette foreground.

## Impact

- `crates/pictura-app/cpp/theme.{h,cpp}` — the `icon` ramp field and token.
- `crates/pictura-app/cpp/svg_icon_engine.{h,cpp}` — `colorFor()` reads the
  token; the disabled branch is unchanged.
- `crates/pictura-app/cpp/design_system_dialog_pages.cpp` — the token appears in
  the palette page.
- `crates/pictura-app/cpp/tests/tst_icon_assets.cpp` — normal, disabled, and
  light-family tint coverage.
- `crates/pictura-app/cpp/tests/tst_design_system.cpp` — token contract and the
  paint-only token set.
- `design-system.html` — regenerated from the theme dump.
- No dependency, data-format, or API changes.

Closes #315.
