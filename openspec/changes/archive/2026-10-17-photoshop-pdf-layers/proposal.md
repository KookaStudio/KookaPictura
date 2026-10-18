# Proposal: photoshop-pdf-layers

## Why

Owner feedback on #627 (PR #727): a PDF that Photoshop saved with Preserve
Photoshop Editing Capabilities opens in Photoshop with its layers (a
Background and a live type layer), but Pictura imports it flattened through
the Import PDF dialog. Only Pictura's own Photoshop PDFs (#626) reopened with
layers.

## What Changes

- `pictura_codec::pdf::photoshop_psd` rebuilds a PSD from the editing data
  Photoshop stores under the first page's `/PieceInfo /AdobePhotoshop
  /Private`: `/ImageResources` (a PSD image-resources section),
  `/StandardImageFileData` (the layer data as the tagged blocks a layered TIFF
  carries: `Layr`, `LMsk`, then additional layer information), and
  `/CompositeImage` (the flattened image; its size, colour space, and depth
  give the PSD header). The layout was read from a file Photoshop saved; Adobe
  does not publish it.
- The PDF object reader learns TIFF predictor 2, which the composite uses.
- `pdf_open` tries Pictura's payload, then Photoshop's data, before the Import
  PDF dialog, so such a file opens with its layers and type layers stay type
  layers.

## Capabilities

### Modified Capabilities

- `interop/pdf-eps-import`: a new requirement for Photoshop's own PDFs.

## Impact

- No new dependency. Encrypted Photoshop PDFs and Indexed / Bitmap / Duotone /
  Multichannel sources still import flattened (ponytail).
