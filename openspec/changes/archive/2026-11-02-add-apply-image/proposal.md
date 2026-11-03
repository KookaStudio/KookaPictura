# Proposal: add-apply-image

## Why

Issue #609 (from the #486 parity audit). `Image ▸ Apply Image…` is an inert
catalogue row. `docs/08-selection/channel-based-masking.md` (SEL-013) specifies
the command: blend a layer and channel of an open document into the active
target, through an optional mask.

## What Changes

- **`pictura-render`**: an `apply_image` module: source and mask planes read at
  document size and native depth, Add and Subtract with Scale and Offset, the
  layer blend modes, and a position-aware kernel that writes 8-bit channels or
  the authoritative 16/32-bit store.
- **`pictura-app`**: an `image_apply_image` bridge, `ActiveOp::ApplyImage` on
  the shared preview path (layer and layer-mask targets), and a modal
  `ApplyImageDialog`.
- **`pictura-script`**: `image.apply.image` is implemented.

## Capabilities

### Added Capabilities

- `imaging/apply-image`

### Modified Capabilities

None.

## Impact

- Crates: `pictura-render`, `pictura-app`, `pictura-script`. No new dependency.
- The catalogue golden changes for the one row.
- `docs/` records the Multiply default and the high-depth Offset rule.
- Calculations (#610) is out of scope.

## Provenance

CS6 Help as cited in SEL-013. The Multiply default comes from a screenshot of
the Photoshop dialog in issue #609. High-depth Scale and Offset are an
inference, not verified parity.
