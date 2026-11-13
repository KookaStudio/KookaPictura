## ADDED Requirements

### Requirement: A spilling transform reports progress as it goes

A tiled transform resample running under a job SHALL advance the job's progress
after every strip, including while its output tiles are being spilled to the
scratch disk, so the progress window keeps moving through a commit larger than
RAM.

#### Scenario: Progress climbs while tiles spill [oct_progress_spill]

- **WHEN** a multi-strip layer is resampled under a job with a scratch volume
  configured, and an eviction pass runs at the start of each strip
- **THEN** the job's progress takes at least four increasing values between 0
  and 1000 during the resample, tiles are spilled before it ends, and the
  output equals the dense resampler's
