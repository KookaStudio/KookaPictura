# Proposal

## Why

The Split Toning tab does not behave like Camera Raw. Its Balance sign is
inverted: the code moves the crossover toward the highlights for a negative
balance, while Camera Raw documents `+ to Highlights, − to Shadows`. The tint
also shifts a pixel's luminance instead of recolouring it, so the result is
brighter or darker than the source. The Saturation tracks are plain grey, where
Camera Raw tints each one from neutral grey to its paired Hue colour, and the
Hue tracks use a coarse seven-stop wheel.

## What Changes

- Make a positive Balance bias the highlight tint and a negative Balance the
  shadow tint, matching the documented direction.
- Remove the tint's own luminance before applying it, so split toning recolours
  without changing the pixel's brightness.
- Use a smooth crossover instead of the linear one.
- Give the Hue tracks a smooth 13-stop hue wheel and each Saturation track a
  grey-to-hue ramp that follows its Hue slider.

The settings keys, ranges, and the Split Toning controls are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the Split Toning requirement is added, covering the
  balance direction, luminance preservation, and the slider ramps.
