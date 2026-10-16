# Proposal: image-mode-conversion

## Why

`Image ▸ Mode` only worked for one direction of one axis: 32 → 16/8 bits
through the HDR Conversion dialog. Every color-mode entry, and 8/16 → 16/32
bits, was a disabled stub, so a document could not be converted at all. This
change ports photorust's Mode submenu (`Document::set_color_mode`,
`convert_to_indexed`, `set_bit_depth`, the Indexed Color dialog, and the
MainWindow prompts) onto this engine.

photorust only relabels the document for Bitmap, Duotone, Lab, and Multichannel
(no pixel or file change) and keeps an Indexed image as RGBA with no palette.
Here each ported conversion is real end to end. The working pixels stay 8-bit
RGB or Grayscale, as they do everywhere else in the engine. The target mode or
depth is carried by the `source_mode` / `source_planes` / `source_palette` /
`source_depth` stores that the PSD writer already re-emits for opened
CMYK/Lab/Indexed/Bitmap/16/32-bit files. A converted document therefore saves
in its new mode and depth.

## What Changes

- `pictura-render::document_ops::mode` (new):
  - `convert_mode` covers Grayscale, RGB, CMYK, and Lab.
    - Grayscale uses Rec. 601 luma, rounded. It is computed at native depth
      when a 16/32-bit store is retained.
    - RGB replicates the gray plane or drops the recorded source mode.
    - CMYK uses photorust's full black-generation (GCR) formula in the codec's
      stored convention.
    - Lab uses the codec's profile-free transform.
  - `convert_to_indexed` flattens, then reuses the Save for Web median-cut
    quantizer and dithers: Exact, Web, Local Perceptual/Selective/Adaptive;
    None, Diffusion, Pattern, and Noise with an amount. It retains the palette
    and the index planes.
  - `convert_to_bitmap` flattens 8-bit Grayscale and reduces it with 50%
    Threshold, an 8×8 Bayer pattern, or Floyd–Steinberg diffusion into a flat
    depth-1 store.
  - `convert_bit_depth` handles 8 ↔ 16, 8 → 32, and 16 → 32 by widening or
    narrowing the retained stores. 32 → 16/8 keeps the HDR Conversion path.
  - `can_convert_mode` / `can_convert_depth` implement the IMG-004 conversion
    matrix and depth limits.
  - `document_color_mode` / `document_bit_depth` report the mode and depth as
    the user sees them.
  - `save_view` drops the app's display alpha plane from a source-mode
    document's composite before writing. Before this, an opened CMYK/Lab/
    Indexed/Bitmap document always re-saved as RGB from the app.
- `pictura-codec` exports `bitmap_rows_to_rgb`, `indexed_to_rgb`,
  `cmyk_to_rgb`, `lab_to_rgb`, and `rgb_to_lab`.
- `pictura-app`:
  - The new bridge `image_adjust/image_mode.rs` exposes the mode and depth
    query/convert calls plus the Indexed preview (`mode_preview` state).
  - `PictureView::save` writes through `save_view`.
  - The C++ side adds `frame_menus_image_mode.cpp` (handlers, check marks, the
    Discard / Flatten prompts), `indexed_color_dialog.*`, and
    `bitmap_mode_dialog.*`.
  - The Mode entries are now checkable commands with frozen ids. Color Table
    moves below the depth group, as in CS6.
- Tests:
  - `document_ops/mode/tests.rs` covers each conversion through a
    `write_psd` → `read_psd` round trip, plus IMG-004 acceptance criteria 3, 4, and 6.
  - The bridge unit tests cover the option mapping.
  - The Qt Test `tst_image_mode` covers menu state, undo, preview/cancel,
    save/reopen per mode and depth, the dialogs, and acceptance
    criterion 1 (undo restores the exact RGB).

## Capabilities

### New Capabilities

- `color/image-mode-conversion`: the engine conversions, their availability
  matrix, and the save behavior.
- `ui/image-mode-menu`: the `Image ▸ Mode` submenu state, prompts, dialogs,
  and the single history state.

### Modified Capabilities

- `color/hdr-conversion`: the 8/16 Bits/Channel commands are no longer gated on
  a 32-bit document. Only a 32-bit source routes through the HDR Conversion
  dialog.

## Impact

- `pictura-codec` (exports only), `pictura-render`, `pictura-app` (bridge,
  C++). No new dependency. New `.cpp`/`.h` files are listed in `CMakeLists.txt`,
  and the Qt Test in `cpp/tests/CMakeLists.txt`.

## Provenance

Source: https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/document.rs
(`set_color_mode`, `convert_to_indexed`, `set_bit_depth`),
`shell/src/MainWindow.cpp` (the Mode submenu), and
`shell/src/dialogs/IndexedColorDialog.cpp`.
Co-authored-by: Zawaro <zawaroarts@gmail.com>

Ceilings (`ponytail:`):

- Duotone, Multichannel, and the Color Table stay disabled stubs. There is no
  ink/plate model to author.
- CMYK/Lab conversions are profile-free (no ICC working space or gamut
  clipping). A 16-bit CMYK/Lab conversion passes through the 8-bit working
  planes.
- The Grayscale weights are inferred.
- Bitmap has no Output resolution, Halftone Screen, or Custom Pattern; Pattern
  Dither is Bayer. A Bitmap document is edited as RGB and an edit saves RGB.
- Indexed has no System/Uniform/Master/Custom/Previous palettes, Forced colors,
  Transparency, or Matte.
- The `Convert Mode` history label is inferred.
