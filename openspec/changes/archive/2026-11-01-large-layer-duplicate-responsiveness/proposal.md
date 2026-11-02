# Proposal

Tracking issue: #1123.

## Why

Duplicating the Background of a 16507×16196 image blocks the GUI thread for
about 25 s, long enough for the desktop to report "not responding". Measured on
the real file through `scripts/profile-tools.py`, the composite is about 4 s of
that. The rest is the Move tool's controls box: `trim_bounds` reads the alpha
plane once per pixel through `Deref`, and each read walks every band in
`Bands::diverged`. A Background has no alpha plane and escapes the scan, so the
stall first appears on its copy, and then on every history step of a layer with
alpha, undo included.

## What Changes

- `Bands::diverged` becomes O(1), so a contiguous read of a banded plane costs
  the same as one of a flat plane.
- The content-bounds scans read rows, in parallel, without materializing a
  diverged plane.
- A duplicate that cannot change the picture returns `Edit::Inserted` and skips
  the composite and display rebuild, at both duplicate sites.
- Undo stops re-activating the Move tool's preview warm, a full-layer bake that
  sampling found on the GUI thread after every history restore.
- `release_background` fills its alpha plane in parallel, without the second
  copy.
- `scripts/profile-tools.py` gains duplicate, move and delete scenarios and RSS.

## Capabilities

### New Capabilities

- `document/layer-duplicate-responsiveness`

## Impact

`pictura-core::plane`, `pictura-render` layer ops, `pictura-app` duplicate edit
sites, `scripts/profile-tools.py`. No file format or golden change.
