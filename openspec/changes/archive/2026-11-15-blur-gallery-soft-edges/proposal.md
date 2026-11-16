# Proposal

Tracking issue: #458.

## Why

Field Blur (and the Blur Effects shared by Iris Blur and Tilt-Shift) leaves a
bright rim around a high-contrast subject instead of the soft edge Photoshop
shows. The Blur Effects run on the already-blurred pixels, so the blurred
midtone edge of a dark subject is brightened more than the subject's interior;
the hard Light Range threshold then ends that brightening on a contour. The
result is a sharp halo that reads as a hard border around the blur.

## What Changes

- Light Bokeh and Bokeh Color are applied to the source colour **before** the
  variable blur, gated by the same per-pixel field weight. A blurred edge then
  keeps the unlit blur's shape instead of gaining a rim.
- The Light Range becomes a soft tonal window that tapers to zero at both
  bounds, so it adds no tonal step at the range edge.
- The Noise pass stays after the blur, unchanged.
- Two engine tests pin the behavior: a blurred edge is not brightened more than
  its interior, and the Light Range bound adds no step.

## Capabilities

### Modified Capabilities

- `imaging/blur-gallery`: the shared Blur Effects render before the variable
  blur and use a soft Light Range window.

## Impact

- `crates/pictura-filters/src/blur/field.rs`: the bokeh pass moves ahead of the
  pyramid; the noise pass is split out; the Light Range gate is softened.
- `crates/pictura-filters/src/blur/{gallery,tilt,path,spin}.rs`: call the split
  passes.
- `crates/pictura-filters/tests/kernel_digests.rs`: the Field, Iris, Tilt-Shift,
  and combined gallery digests move with the intentional render-order change.
