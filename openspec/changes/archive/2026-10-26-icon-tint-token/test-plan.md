# Test Plan

SVG icons render through `SvgIconEngine`, which replaces the asset's alpha with
one tint at 80 % opacity. The plan checks the new dedicated token on the dark
and light families, the disabled treatment, and the theme-change cache key.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lia_foreground` | Apply the default dark theme, render `tool.brush`, find the strongest pixel | RGB is `#FFFFFF`; alpha is ~204 (80 % of 255) | `tst_icon_assets::defaultTintIsWhite` |
| `lia_disabled` | Render `tool.brush` in `QIcon::Disabled` | Strongest pixel is `#686868`, not the white icon tint | `tst_icon_assets::disabledTintStaysDimmed`, `tst_icon_assets::disabledDiffers` |
| `lia_light_family` | Apply the light family, render `tool.brush` | Strongest pixel is `#2B2B2B`, legible on light surfaces | `tst_icon_assets::lightFamilyTintIsDark` |
| `lia_theme_change` | Render an icon, apply another brightness level, compare the generation | The palette generation advanced, so the cache key changes | `tst_icon_assets::themeChangeRerenders` |
| `ds_tokens_shape` | Read `Theme::tokens()` at levels 0 and 3 | The `icon` token exists and is `#FFFFFF` on the dark family at every level | `tst_design_system::tokensCoverColoursAndMetrics` |
