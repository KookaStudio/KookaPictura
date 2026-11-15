# Tasks

## 1. Streamed exact readers

- [x] 1.1 Export, Save for Web and raster save formats: bands → sRGB per band → one packed RGBA.
- [x] 1.2 Copy Merged composites the selection bounds only.
- [x] 1.3 Trim scans coverage by band; Duplicate (merged) streams into its one RGBA.
- [x] 1.4 Load Channel Selection and the Color Range commit write their mask by band.
- [x] 1.5 Match Color source statistics accumulate by band (`pictura-adjust` accumulator).

## 2. Tile-sourced selection readers

- [x] 2.1 `pictura-select` image source: a whole buffer or a tile fetch.
- [x] 2.2 Wand, Grow and Similar read the source; app passes the Sample All fetch.
- [x] 2.3 Refine Edge and decontaminate read the source.
- [x] 2.4 The Magnetic Lasso edge map: peak over bands, tile costs from the source on a document clone.

## 3. Reduced-level previews

- [x] 3.1 A level-`k` composite reader for a preview size.
- [x] 3.2 Color Range, Refine Edge and Replace Color previews read it.
- [x] 3.3 `PictureView::preview_image(max_side)`; the control-server capture and the Image Size preview use it.

## 4. Conversions

- [x] 4.1 Grayscale, CMYK, Lab and depth up-conversion stream their RGB source, dithered profile conversions included; `materialize_composite` stays only for conversions that do not stream.

## 5. Backdrops

- [x] 5.1 Liquify composites its frame; Vanishing Point streams into its sRGB output.
- [x] 5.2 Move and transform backdrops patch the hidden layers' influence.

## 6. Verification and close-out

- [x] 6.1 Byte-identity tests and heap guards per test-plan row.
- [x] 6.2 Remove the `ponytail:` markers this resolves; keep the ones design.md lists as out of scope.
- [x] 6.3 `docs/dev/STATE.md` entry; 17000² profile numbers in the PR.
