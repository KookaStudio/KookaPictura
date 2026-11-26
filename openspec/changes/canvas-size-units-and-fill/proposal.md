# Proposal

## Why

Canvas Size lacks Picas and Columns, and offers an extension color on documents
that will grow transparently. Issue #566 tracks these two gaps.

## What Changes

- Add Picas and Columns to the Canvas Size dimension controls. Resolve them
  using document resolution, the point/pica setting, and the shared column
  width and gutter preferences.
- Disable the extension menu and swatch on documents without a Background
  layer, and use that original layer state to decide whether to fill newly
  exposed canvas. Preserve existing layer content and Actions behavior.

## Capabilities

### New Capabilities

- `ui/canvas-size-dialog`: Canvas dimension entry and Background-dependent
  color controls.

## Impact

Canvas Size dialog, stored preferences, and Qt regression coverage. The
editable Units & Rulers column preference controls are supplied by #565.
