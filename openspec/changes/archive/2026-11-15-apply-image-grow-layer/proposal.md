# Grow Apply Image's destination layer

## Why

Issue #1139. Apply Image (#609) writes only inside the destination layer's rect,
so applying an image to a smaller layer leaves the rest of the document
untouched. Photoshop extends the destination across the document when Preserve
Transparency is off. The engine has `layer_bounds::grow_layer_to` (#1208) and the
filter preview already remembers a pre-preview rect, but Apply Image never grows
and its preview baseline would not survive a rect change.

## What Changes

- **Apply Image grows the destination.** With Preserve Transparency off and no
  transparency lock, the destination pixel layer grows to the document before
  the blend, padding the new area with transparent pixels at every depth.
- **Preserve Transparency keeps the rect.** Preserve Transparency or a
  transparency lock leaves the layer rect unchanged.
- **Preview baselines survive growth.** The layer is grown before the preview
  baseline is saved, so a later preview re-applies from the original pixels and
  Cancel shrinks the rect back; previews never compound across a growth.
- **Native depth survives growth.** A 16- or 32-bit store is padded by the
  existing grow primitive and keeps its precision.
- **The kernel's limitation note is removed.** `apply_image_region` still writes
  only within the layer rect; the app now grows first, as it does for filters.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `imaging/apply-image`: a smaller destination layer grows to the document when
  Preserve Transparency is off, and previews restore the pre-preview rect.

## Impact

`pictura-app` (the Apply Image path in `impl_filters`, its preview baseline and
tests), `pictura-render` (the kernel's documented contract and an integration
test), and `docs/dev/STATE.md` / `docs/dev/doc-code-parity-gaps.md` (SEL-1).
