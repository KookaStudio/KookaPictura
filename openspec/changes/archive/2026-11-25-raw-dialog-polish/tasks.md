# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_acr_dialog::panelContentIsInset` covering `rp_panel_inset`
- [x] 1.2 Add `tst_acr_dialog::hslPanelColorsMatchRanges` covering `rp_hsl_ramps`
- [x] 1.3 Add `tst_acr_dialog::effectsTabUsesFxIcon` and `tst_acr_dialog::settingsButtonOpensList` covering `rp_effects_icon` and `rp_settings_button`

## 2. Panel inset

- [x] 2.1 Set every control panel's content margins to eight pixels left and right in `buildControlPanels`

## 3. HSL colours

- [x] 3.1 Replace the full-spectrum Hue ramp and grey Saturation/Luminance ramps with eight per-range ramps, keeping the Grayscale Mix grey

## 4. Effects icon and fx button

- [x] 4.1 Give the Effects tab the `layers.fx` icon and remove the Presets tab from the strip
- [x] 4.2 Move the settings list into an `acrSettingsButton` popup at the right of the tab strip, keeping the existing actions

## 5. Docs and verification

- [x] 5.1 Update `public_docs/pictura-raw.md` for the eight tabs, fx button, and settings list
- [x] 5.2 Run the focused Qt suites, fmt, file-size, guard, test-plan, and OpenSpec validation
- [x] 5.3 Archive the change with `openspec archive raw-dialog-polish`
