# Proposal

## Why

Users saving layered PSD/PSB files have no File Handling preference to choose between a broadly readable merged image and a smaller file. Issue #639 adds that choice for workflows that exchange files with Photoshop CS6, Photoshop CC, and applications that rely on merged pixels.

## What Changes

- Add a persistent Maximize PSD and PSB File Compatibility setting with Always, Ask, and Never. Default to Always when no setting is stored.
- Apply the setting to interactive Save, Save As, and Save a Copy to PSD/PSB. Ask resolves one interactive save and supports cancellation before writing or changing its ICC save setting; control-server saves and Action replay retain their noninteractive behavior.
- Preserve the image data needed to reopen layerless documents and retain document-level channels.
- Verify PSD and PSB exchange in both directions with CS6 and current Photoshop CC, with and without a real merged image.

## Capabilities

### New Capabilities

- `interop/psd-psb-compatibility` covers the preference, per-save choices, safe omission, and Photoshop exchange.

### Modified Capabilities

- `compositing/native-depth-save` permits explicit omission and reconstructs a missing native composite on Include.
- `document/document-source-state` permits fixture-supported placeholder serialization while retaining direct writer preconditions.
- `interop/save-format-options` allows the native compatibility prompt when Ask applies.

## Impact

The Preferences dialog and persisted session settings carry the preference. The save coordinator and native worker carry the resolved choice. PSD/PSB staging and codec changes depend on the file structure observed in Adobe-authored fixtures. Verification includes automated Qt and Rust tests and manual Photoshop checks.
