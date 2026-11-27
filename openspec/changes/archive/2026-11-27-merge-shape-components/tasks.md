## 1. Test sources

- [x] 1.1 Add Rust geometry tests for nested ambient winding and parity, boundary tolerance, and no-op inputs.
- [x] 1.2 Add Qt tests for menu routing, Paths-row priority, focused contours, undo, redo, and no-op inputs.
- [x] 1.3 Add private app Rust fixtures for curved renderer geometry and declined imported and empty-result masks.

## 2. Implementation

- [x] 2.1 Normalize interacting contour groups with enclosing context and the target's effective fill rule and geometry, retaining independent and open mask contours.
- [x] 2.2 Enforce the boundary, topology, geometry-budget and mask-rewrite safeguards before mutation; route Path Operations through the active target and record one successful edit.
- [x] 2.3 Update the roadmap and parity entry and archive the change.

## 3. Verification

- [x] 3.1 Run the Rust and Qt suites and repository gates in the scheduled build phase.
