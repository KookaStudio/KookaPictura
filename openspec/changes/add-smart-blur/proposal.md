# Proposal: add-smart-blur

## Why

Issue #339 (part of the #167 remaining-filter audit). `Filter ▸ Blur ▸ Smart
Blur` was a disabled stub with no kernel, and photorust has no source to port,
so this is a new engine. The docs contract (`docs/06-filters/blur-filters.md`)
describes it as an edge-aware selective blur: Radius is the area searched for
dissimilar pixels, Threshold how dissimilar they must be before they are
affected, a Quality control, and three modes: Normal, Edge Only (black-and-white
edges) and Overlay Edge (white edges over the result).

## What Changes

- `pictura-filters` adds `SmartBlurMode` and `Filter::SmartBlur { radius,
  threshold, quality, mode }`: a selective Gaussian over the disc of Radius
  that only averages neighbours whose every channel is within Threshold of the
  centre, Quality setting the sampling density, and an edge map (a pixel with a
  4-neighbour past Threshold) for Edge Only and Overlay Edge.
- `pictura-render` lists Normal mode in `filter_preserves_opacity` and gives
  the kind a `radius + 1` preview apron.
- `pictura-app` maps a `smart-blur` kind of four slots (Radius, Threshold,
  Quality, Mode) and enables `Filter ▸ Blur ▸ Smart Blur…` with CS6's
  controls.
- Ceilings: Adobe's kernel is closed. The joint channel test, the Gaussian
  weighting, Quality's tap budget and the 4-neighbour edge rule are inferred
  from CS6's output, as are the defaults (Radius 3, Threshold 25, Low).

## Capabilities

### New Capabilities

- `imaging/blur-filters` gains *Smart blur*: the selective average, the
  threshold, Quality, the three modes, and the refusals.
- `imaging/filter-app-ui` gains *Smart Blur dialog*: the menu entry, the
  ranges and defaults, the slot order, the preview, and Last Filter.

## Impact

- `pictura-filters` (`blur/smart.rs`, `Filter`, `apply`, oracle mapping,
  digests, profile), `pictura-render` (`filter_preserves_opacity`,
  `preview_apron`), `pictura-app` (`filter_map.rs`, `filter_map_choices.rs`,
  `arities.rs`, `filter_commands.cpp`), `tst_smart_blur`, `tst_filter_menu`.
