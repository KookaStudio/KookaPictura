## 1. Coverage

- [x] 1.1 Write Qt coverage for entry points, preview transactions across the commit delay, undo, profile persistence, factory compatibility, configured Image Auto, and normal, Quick Mask, and layer-mask dispatch.
- [x] 1.2 Write Rust coverage for source-resolution tiled fitting, selection-derived histograms, configured native u16 and f32 correction, and Levels/Curves PSD and PSB round trips.

## 2. Implementation

- [x] 2.1 Add typed Auto defaults to profile preferences and a configurable CPU fit shared by Properties and Image Auto.
- [x] 2.2 Add the dialog, Properties menu and Auto button, and a dialog-owned reversible Levels/Curves preview transaction that suspends and clears ordinary commit timers.
- [x] 2.3 Route configured Image Auto to authoritative native samples explicitly while retaining normal direct, Quick Mask, and layer-mask dispatch, stock output, named algorithms, and selection blending.

## 3. Deferred checks

- [ ] 3.1 Build and run the automated suites, manual CS6 comparison, and project gates after authorization.
