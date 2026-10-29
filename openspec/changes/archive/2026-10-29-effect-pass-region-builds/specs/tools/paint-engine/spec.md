# Spec Delta

## MODIFIED Requirements

### Requirement: Incremental dirty region and dab latency

The paint engine SHALL report an incremental dirty rectangle rather than the
cumulative dirty rectangle of the whole stroke, so compositing after a dab
touches only the pixels that dab changed. The rectangle a present carries SHALL
cover the footprints of the dabs placed since the previous present, grown by the
painted layer's paint-change reach (outer and inner effects) and, for a mask
stroke, the mask feather reach, and MUST NOT grow with the dabs of earlier
presents in the same stroke. A reduced-resolution preview present is exempt: it
carries the level-space extent, and the exact commit repairs the composite. The
CPU region compositor SHALL composite only the requested region rather than
fully compositing the document and slicing the result, and a region blit SHALL
update only that region of the presented image instead of invalidating the whole
scaled present cache. The sustained cost of a brush dab on a 4000×4000 document
SHALL meet the canvas-view budget (`docs/dev/canvas-view-spec.md`):
input-to-first-pixel at most 16 ms per dab and a sustained redraw at or above
60 FPS.

#### Scenario: A dab dirties only its own region [lpe_dirty_per_dab]

- **WHEN** a stroke places a dab after other dabs
- **THEN** the reported dirty rectangle covers the dabs since the previous
  present, not the union of all prior dabs of the stroke

#### Scenario: A present covers only the dabs since the previous present
[lpe_dirty_since_last_present]

- **WHEN** several dabs are placed between two presents of the same stroke
- **THEN** the reported dirty rectangle is no larger than the union of those
  dabs' footprints grown by the painted layer's paint-change reach and, for a
  mask stroke, the mask feather reach

#### Scenario: Region compositing matches the full composite [lpe_region_equal]

- **WHEN** a region is composited through the CPU region compositor
- **THEN** every pixel in that region equals the full-document composite within
  the compositor tolerance

#### Scenario: A styled-layer present refreshes the halo [lpe_present_effect_reach]

- **WHEN** a dab is painted on a layer carrying a layer style, or on a feathered
  mask
- **THEN** the present region includes the paint-change reach (and the mask
  feather reach), so the moved halo or feather ramp is refreshed rather than
  left stale

#### Scenario: A dab meets the canvas-view budget [lpe_dab_budget]

- **WHEN** a brush dab is processed on a 4000×4000 document
- **THEN** the input-to-first-pixel latency is at most 16 ms and the sustained
  update rate is at least 60 FPS, measured against the canvas-view budget
