# Measurement Scale, Presets, and Export Writers

## Why

The Ruler measures in pixels, and the application cannot apply or restore a document’s measurement calibration. Issue [#635](https://github.com/KookaStudio/KookaPictura/issues/635) adds calibration and the writers needed to export measurements recorded by the Measurement Log.

## What Changes

- In Extended, add Default, Custom, and named presets under Set Measurement Scale. Users can type calibration lengths or derive the pixel length from the Ruler, read a calibrated distance, see the active scale in the Info panel and document status, and undo changes to a document’s active scale.
- Save the active calibration with the PSD or PSB via image resource 1074. Preserve the original resource bytes on files whose calibration has not been edited, including files saved in Standard.
- Provide standalone writers for UTF-8 tab-delimited measurement rows and numbered histogram CSV files. They consume recorded values with their original units and scale.
- Use issue [#510](https://github.com/KookaStudio/KookaPictura/issues/510) for the session log, recording and data-point selection, and connecting the Log’s Export action to these writers.

## Capabilities

### New Capabilities

- `tools/measurement-scale` covers document calibration, application presets, and the Extended calibration controls.
- `codec/psd-measurement-scale` covers resource 1074 and opaque preservation.
- `interop/measurement-log-export` covers serialization of selected measurements and histogram artifacts.

### Modified Capabilities

## Impact

The application needs calibration actions, a dialog, status readouts, and document history integration. The core supplies the scale value and conversions. The codec supplies PSD/PSB persistence, and the writers give #510 a file output interface independent of Qt. No new dependency is expected.
