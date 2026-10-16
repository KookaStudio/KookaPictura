# Proposal: ocean-ripple-refraction

## Why

Kooka's Ocean Ripple sums eight smooth sinusoids of about a pixel's
amplitude. At CS6's defaults (Size 9, Magnitude 9) the picture barely
changes. CS6 makes it look as if seen through rippled glass: frosted blobs at
small sizes, and edges broken into scattered fragments at large magnitudes.
Issue #286 reports the mismatch.

## What Changes

- `distort::ocean_ripple` refracts through a seeded bumpy surface. That
  surface is smoothstep value noise on a lattice whose cell is
  `3 + 0.5·size` px. Each pixel samples the source displaced along the
  surface's slope by `0.15·magnitude^1.5·√(cell/4)`. Sampling stays bilinear
  with clamp-to-edge, Magnitude 0 is still a no-op, and the ranges are
  unchanged.
- The `ocean-ripple` kind and dialog default Magnitude to 9, which is CS6's
  default. It was 5, which the corpus had only inferred.

## Capabilities

### New Capabilities

- `imaging/ocean-ripple`: the refraction model, how its reach depends on
  Magnitude, and the defaults.

## Impact

- `pictura-filters` (`distort/ripples.rs`) and `pictura-app` (default
  Magnitude). No new dependency.
- **Output changes:** every Ocean Ripple result changes. No golden baseline
  covers it.
- **Oracle:** none. The tests README marks the ImageMagick `-wave` measurement
  as belonging to the old model.
- **Docs:** `docs/06-filters/distort-filters.md` still lists Magnitude 5
  *(inferred)*, and needs a separate `TASK-ALLOWS-DOCS` change.
