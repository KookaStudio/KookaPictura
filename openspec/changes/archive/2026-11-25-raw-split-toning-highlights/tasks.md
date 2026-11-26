# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura-adjust::split_toning_highlights_only_tints_the_bright_end` covering `rs_highlights_only`
- [x] 1.2 Add `tst_acr_dialog::splitToningHighlightsChangeThePreview` covering `rs_highlights_only`

## 2. Render

- [x] 2.1 Blend the pixel toward the tint colour at its own lightness, preserving luminance

## 3. Verification

- [x] 3.1 Run the split-toning tests, the focused Qt suites, clippy, fmt, file-size, guard, test-plan, and OpenSpec validation
- [x] 3.2 Archive the change with `openspec archive raw-split-toning-highlights`
