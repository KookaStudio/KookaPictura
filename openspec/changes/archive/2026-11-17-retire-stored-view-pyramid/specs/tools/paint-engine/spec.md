## MODIFIED Requirements

### Requirement: Reduced-resolution preview for large dabs

When a dab's bounding box would push its rasterization past the
input-to-first-pixel budget, the system SHALL present a reduced-resolution
preview of the in-progress stroke instead of rasterizing it at full resolution.
The preview SHALL cover the stroke's bounding box at a tile-pyramid level whose
own rasterization and present fit the same budget as the full-resolution
rasterization it replaces, SHALL be built from a snapshot of that level taken at
stroke start rather than from a fresh scan of the document, and MUST NOT write
level 0. Dabs whose bounding box fits the budget SHALL rasterize at full
resolution exactly as before.

#### Scenario: A small dab is not previewed [pe_preview_below_threshold]

- **WHEN** a stroke runs with a dab bounding box at or under the raster budget
- **THEN** it rasterizes and presents at full resolution, with no reduced
  document and no change to the pixels it produces

#### Scenario: A large dab is previewed [pe_preview_above_threshold]

- **WHEN** a stroke runs with a dab bounding box above the raster budget
- **THEN** the in-progress stroke is presented at a tile-pyramid level whose own
  work fits the budget, built from a snapshot taken at stroke start rather than
  a scan of the document, and the number of pixels rasterized per dab no longer
  grows with the bounding box

#### Scenario: The preview does not touch the exact stroke's pixels [pe_preview_is_presentational]

- **WHEN** a previewed stroke is recorded
- **THEN** the reduced-resolution work is discarded, the exact full-resolution
  stroke is computed before the history state is recorded, and every pixel of
  the committed document is byte-identical to the same stroke run without a
  preview

#### Scenario: Cancelling a previewed stroke restores the document [pe_preview_cancel]

- **WHEN** a previewed stroke is cancelled
- **THEN** the recorded samples are dropped without producing a history state
  and the displayed document is the pre-stroke one

## REMOVED Requirements

### Requirement: The live stroke presents and rebuilds the pyramid frame-bounded

**Reason**: A whole stored-pyramid rebuild is not the production frame operation.

**Migration**: Use The live stroke refreshes display tiles per frame; existing scenario tags remain traceable in the replacement.

## ADDED Requirements

### Requirement: The live stroke refreshes display tiles per frame

While a stroke is live the app SHALL present at most once per frame: the dab
that opens a frame presents immediately and the frame's later dabs accumulate
into one pending region, shown at the frame flush. The affected cached tiles SHALL be
repaired or invalidated once per frame flush rather than once per dab, and each
exact tile presented after the flush MUST equal an independent whole rebuild of
the current working document at that level.

#### Scenario: A frame of dabs refreshes the affected tiles once [pe_pyramid_per_frame]

- **WHEN** several dabs arrive between two presents of a live stroke
- **THEN** the pending tile damage is refreshed once, at the flush, rather than once per dab

#### Scenario: The refreshed pyramid matches a rebuild [pe_pyramid_flush_equal]

- **WHEN** a frame's accumulated region is folded into the pyramid
- **THEN** every exact tile presented equals a rebuild from the frame's current working document
