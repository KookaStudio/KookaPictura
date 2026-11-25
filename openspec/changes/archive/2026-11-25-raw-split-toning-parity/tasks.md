# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura-adjust::split_toning_balance_biases_the_highlights` covering `rs_balance`
- [x] 1.2 Add `pictura-adjust::split_toning_preserves_luminance` covering `rs_luminance`
- [x] 1.3 Add `tst_acr_dialog::splitToningSaturationRampsFollowHue` covering `rs_saturation_ramp`

## 2. Split toning render

- [x] 2.1 Invert the Balance crossover so positive biases highlights and negative shadows
- [x] 2.2 Remove the tint's luminance and use a smooth crossover

## 3. Split toning panel

- [x] 3.1 Give the Hue sliders a 13-stop hue wheel and the Saturation sliders a grey-to-hue ramp that follows the Hue

## 4. Docs and verification

- [x] 4.1 Update `public_docs/pictura-raw.md` for the balance direction and tinted Saturation tracks
- [x] 4.2 Run the focused suites, fmt, clippy, file-size, guard, test-plan, and OpenSpec validation
- [x] 4.3 Archive the change with `openspec archive raw-split-toning-parity`
