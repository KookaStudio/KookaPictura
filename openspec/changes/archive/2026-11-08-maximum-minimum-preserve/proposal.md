# Maximum and Minimum preserve and radius

## Why

`Filter ▸ Other ▸ Maximum…` and `Minimum…` clamp their radius at 100 px and
expose no shape control. Issue #364 asks for the modern Photoshop dialog: a
radius up to 500 px, a warning above 100 px that the operation can be slow, and
a `Preserve` menu with `Squareness` and `Roundness`. The current kernel is
square only, so a mask user cannot choose the round footprint.

## What Changes

- The engine clamp for `Filter::Maximum` and `Filter::Minimum` rises from 100
  to 500 px; a request above 500 still clamps and never errors.
- Both variants carry a `MorphologyPreserve` choice. `Squareness` keeps the
  existing `(2r+1)²` square footprint; `Roundness` uses the disk footprint
  `dx² + dy² <= r²`.
- The `maximum` and `minimum` descriptors gain a `Preserve:` choice after
  `Radius:`, defaulting to `Squareness`, and their slot builders read it.
- The shared filter dialog shows an inline, non-blocking warning while the
  radius is above 100 px.
- The GPU square morphology keeps the accelerated path; the round footprint
  declines to the CPU oracle.
- `docs/06-filters/other-filters.md`, `public_docs/filter-video-other-digimarc.md`,
  `ROADMAP.md`, `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md`
  record the extension.

## Capabilities

- `imaging/other-filters` — the Maximum/Minimum engine requirement gains the
  larger clamp and the two footprints.
- `imaging/filter-app-ui` — a new requirement for the Maximum/Minimum dialog's
  radius, Preserve menu, warning, and commit.

## Impact

- `crates/pictura-core` — a disk dilation/erosion kernel beside the square one.
- `crates/pictura-filters` — the enum, the variant fields, the engine dispatch,
  the catalogue rows and builders, and the tests.
- `crates/pictura-render` — the GPU plan declines the round footprint.
- `crates/pictura-app` — the warning bridge and the dialog label, plus a Qt
  test.
- No C++ menu code: the descriptor still drives the menu.
