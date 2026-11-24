# Proposal: colour for the Canon EOS R8 Mark II and the remaining Canon bodies (#1583)

## Why

#1579 left Canon bodies without colour or on borrowed matrices. The EOS R8
Mark II's only sample fitted at 4.71 ΔE median, over the 4.0 limit, so it
still refuses to open. The EOS M6 Mark II and PowerShot G7 X Mark III take
same-sensor proxies (the 90D's and the G5 X Mark II's) because their
raw.pixls.us samples are too grey to fit. The PowerShot G1 X Mark III was
listed as an uncovered CR3 body; it records CR2, and the CR2 reader neither
has a matrix for it nor reads its as-shot white balance, because its maker
note stores the colour data as UNDEFINED bytes rather than SHORT values.

## Owner decisions (2026-11-24)

- **Samples.** Owner-supplied files in `~/Downloads/PHOTOSHOP/RAW/Canon`
  (from the dpreview sample galleries): two more R8 Mark II CR3s, two M6
  Mark II and two G7 X Mark III CR3s, and two G1 X Mark III CR2s. They are fit
  inputs only and are not committed; `camera-jpeg-fits.json` records each
  file's name and SHA-256.
- **R8 Mark II fallback.** If no sample fits, take the EOS R8's row as a
  proxy. Not needed: a new sample fits.
- **G1 X Mark III.** Extend the camera-JPEG fit to CR2 in this change.
- **Scope.** Every body a usable sample exists for; the change closes #1583.

## What changes

- The CR2 reader reads Canon colour data stored as BYTE or UNDEFINED as
  16-bit words in the file's byte order, so the G1 X Mark III reports its
  as-shot white balance.
- `scripts/fit-camera-jpeg-matrices.py` reads CR2s (IFD0's full-size JPEG
  strip, the Exif maker note's colour data) and colour-data version -4,
  whose 8-word preset records start Daylight at word 103.
- New fits give the EOS R8 Mark II, EOS M6 Mark II and PowerShot G7 X Mark III
  their own D55 + Tungsten rows, and the PowerShot G1 X Mark III a D55 row.
  The two camera-JPEG proxies and their generator code are removed.
- Tests that needed a colourless CR3 body name the EOS Ra.

## Capabilities

- Modified: `codec/camera-matrices`.
