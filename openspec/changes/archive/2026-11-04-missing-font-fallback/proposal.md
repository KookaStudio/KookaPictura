# Proposal

## Why

Issue #1171. A type layer whose font family is missing can paint its text
bounding box as an opaque fill instead of rendering the text. The missing-font
alert and the Layers-row mark already name the original family, and
`Type > Replace All Missing Fonts` is a manual command.

The renderer resolves an unregistered family to the bundled Liberation Sans and
renders text, so the normal missing-font path already works. The opaque fill
appears when the resolved face paints no glyph pixel: `Body::TypeSource` then
falls through to `composite_pixels`, and a channel-less layer paints an opaque
rect over its layer rect. A host-registered substitute face for the missing
family can trigger that path.

## What Changes

- **A face that paints nothing retries with the bundled face.** The text
  renderer lays out and paints with the resolved face; when no glyph pixel
  paints and the resolved face is not the bundled Liberation Sans, it retries
  the layout and paint with the bundled face.
- **A proxy-less type layer that paints nothing is transparent.** The CPU
  compositor leaves the backdrop unchanged instead of painting the layer's
  channel-less rect.
- **The substitution is render-time only.** The layer's stored family, its
  `TySh` data, and the missing-font detection keep the original family, so the
  warning still names what is missing.

## Capabilities

### New Capabilities

- `tools/missing-font-fallback`

### Modified Capabilities

- `tools/type-live-composite`: a proxy-less type layer whose text paints no
  pixel leaves the backdrop unchanged.

## Impact

`pictura-render` (`text_render`, `composite`). No C++ change: the missing-font
alert and the Layers-row mark are unchanged.
