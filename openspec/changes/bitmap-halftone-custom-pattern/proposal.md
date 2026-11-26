## Why

Bitmap conversion currently offers only three of the five documented methods. Pattern Dither uses a Bayer approximation; its CS6 geometry is explicitly deferred under #556 because no reproducible CS6 oracle is available and the existing capability requires an ordered 8×8 matrix. Halftone Screen and Custom Pattern let users make printable one-bit artwork with a regular screen or a chosen texture (#556).

## What Changes

- Add Halftone Screen with frequency units, angle, and six spot shapes.
- Add Custom Pattern with a picker for existing built-in, embedded, and imported patterns.
- Carry the method options through command recording and replay.

## Capabilities

### Modified Capabilities

- `color/image-mode-conversion` gains both Bitmap conversion methods.

## Impact

The render conversion, Image Mode bridge, Bitmap dialog and command wiring, and their existing tests change. No new dependencies are required.
