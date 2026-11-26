# Proposal

## Why

Issue #1500's follow-up reports four gaps in the Pictura Raw dialog. The bottom
controls still do not share a baseline: the preview's zoom/Fit and
Shadows/Highlights row keeps a 4 px bottom inset, so it sits above the right
panel's Done/Cancel/OK row. The dialog has no visible resize grip. The histogram
shows only the preview's average RGB; it does not follow the pixel under the
cursor and does not show the camera's f-number, shutter, ISO, or focal length.
The dialog never shows the photo's name, dimensions, resolution, colour space,
or bit depth, which the reference Camera Raw dialog puts in a status bar.

## What Changes

- Put the zoom/Fit, Shadows/Highlights, and Done/Cancel/OK controls on one
  baseline and add a full-width status bar beneath them with a size grip in the
  bottom-right corner.
- Show the photo's name, colour space, bit depth, dimensions with megapixels,
  resolution in ppi, and file size in the status bar, for the pending raw file
  and for the open document.
- Show the camera's f-number, exposure time, ISO, and focal length beside the
  histogram readout when the raw or document metadata carries them.
- Make the histogram's RGB readout follow the pixel under the cursor while the
  pointer is over the preview, and restore the average when it leaves.
- Add one free bridge function that reports the photo information as
  `key<TAB>value` rows and parse the pending raw file's EXIF for the exposure
  tags.

No render, settings-codec, or smart-filter behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the dialog gains a photo-information status bar, camera
  exposure readout, a hover picker, and an aligned bottom bar with a size grip.
  The requirements are additive; the existing tabs, preview, settings actions,
  and OK/Cancel contracts are unchanged.
