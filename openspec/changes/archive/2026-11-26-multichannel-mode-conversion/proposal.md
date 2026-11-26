# Proposal

## Why

Image › Mode › Multichannel is disabled, and the PSD codec can only round-trip a limited set of imported Multichannel files. Issue #554 calls for a conversion that produces independent spot plates users can inspect and save.

## What Changes

- Convert supported 8- and 16-bit RGB, Grayscale, CMYK, and Lab documents to layerless Multichannel in one undoable operation.
- Present the converted source component planes as named spot channels and preserve existing document-level extra channels.
- Save and reopen the resulting plate data in PSD and PSB at the conversion depth.
- Keep Duotone ink authoring, deleting a process channel to enter Multichannel, and DCS export outside this change.

## Capabilities

### Modified Capabilities

- `ui/image-mode-menu` enables, prompts for, and records Multichannel conversion.
- `codec/psd-color-modes` adds conversion and persistence for named Multichannel plates.
