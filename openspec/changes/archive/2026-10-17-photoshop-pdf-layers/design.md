# Design: photoshop-pdf-layers

## Decisions

### D1 Rebuild a PSD, then reuse the PSD reader

The editing data is a PSD in pieces, so the rebuild assembles the PSD
sections (header from the composite, empty colour-mode data, the image
resources as stored, a layer-and-mask section whose layer info is `Layr`,
global mask `LMsk`, and every other block as additional layer information,
then the composite as raw planar image data) and hands the bytes to
`read_psd_with`. Type layers, masks, effects, and smart objects then come back
exactly as the PSD reader already reads them.

### D2 Block layout

`/StandardImageFileData` is a run of `8BIM` (or `8B64`) tagged blocks, each
`signature, key, u32 length, data`, the next block starting at the following
4-byte boundary. Observed keys: `Layr`, `LMsk`, `Patt`, `Txt2`, `FMsk`. A
16-bit file carries its layers in `Lr16`, which passes through as additional
layer information with empty layer info, as in a 16-bit PSD.

### D3 Composite

`/CompositeImage` is an image XObject, here Flate with TIFF predictor 2. Its
colour space gives the mode (DeviceGray 1, DeviceRGB 3, DeviceCMYK 4, Lab 9,
ICCBased by `/N`); DeviceCMYK samples are inverted into PSD's ink convention.
The reader recomposites from the layers on open, so the composite matters for
flat files and thumbnails.

ponytail: only an RGB file from Photoshop was available; Gray, CMYK, Lab, and
16-bit follow the same layout by inference.

### D4 Order

`pdf_open`: Pictura's payload, then Photoshop's data, then the Import PDF
dialog. The PDF's path stays the document's path, so Save writes a (Pictura)
Photoshop PDF through the Save Adobe PDF dialog, as for #626 files.
