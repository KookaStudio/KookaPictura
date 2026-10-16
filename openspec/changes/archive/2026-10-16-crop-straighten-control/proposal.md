# Proposal

## Why

Issue #102: the Crop tool's Straighten control was a bare icon with no label,
`Ctrl` did not invoke it, a near-vertical or right-to-left reference line rotated
the image by ~90° or ~180°, and committing a straightened full-canvas box left
background-filled corners (parity criterion 6 of `docs/03-tools/crop-tool.md`).

## What Changes

- The Straighten toggle shows its `Straighten` label to the right of the icon;
  either toggles the line.
- Holding `Ctrl`/`Cmd` arms the line until the key is released.
- The reference-line angle levels against the nearer axis, in (-45°, 45°], and
  adds to the session's current angle.
- A finished line shrinks the box about its centre to fit inside the rotated
  image, so the commit has no empty corners.
- The image holds still while the line is drawn and rotates when it is
  released, as in CS6.
- The control's icon is a spirit level (`crop.straighten`, original artwork).
- Drawing the line shows the `∠: <angle>°` readout; the armed line has its own
  crosshair-and-spirit-level cursor (`cursor.straighten`, original artwork).
- **BREAKING**: none.

## Capabilities

- Modified `tools/crop-straighten` (Straighten model; Crop pointer cursors).
- Added requirements in `tools/crop-straighten`: Straighten control, Straighten
  reference line angle, Straighten box fit.
