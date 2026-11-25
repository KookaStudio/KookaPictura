# Proposal

## Why

With Highlights Hue 230 and Saturation 52 on a normally exposed raw image, the
bell in the preview kept its original colour: the tint only reached pixels near
white, while the lit surface sat in the upper midtones. The render blended
toward the tint, but the crossover started at the 50% luminance pivot, so a
moderate saturation changed the upper midtones by only a few levels. The same
weakness applied to shadows below the pivot.

## What Changes

- Overlap the shadow and highlight tints across a crossover centred on the
  Balance pivot, instead of starting each tint only at the pivot, so a moderate
  saturation visibly tints the upper (or lower) midtones.
- Tint at the hue's full saturation and let the Saturation slider scale the
  blend amount, so a mid-range saturation keeps the selected hue.
- Desaturate toward the neutral of the same luminance when the scaled colour
  would clip, keeping the tonal value instead of clipping the peak channel.
- Cover the case with a render test at the reported values and tighten the
  dialog preview test to a mid-grey layer and a measurable channel shift.

The settings keys, ranges, and the Split Toning controls are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the Split Toning requirement now states that the two
  tints overlap across the midtones and adds the `rs_midtone_visibility`
  scenario.
