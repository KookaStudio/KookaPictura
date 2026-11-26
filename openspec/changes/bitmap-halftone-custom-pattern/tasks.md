## 1. Regression tests

- [x] 1.1 Add render coverage for periodicity, all shapes, rotation, equivalent units, boundaries, endpoints, depth-1 output, tiling, and missing or invalid options.
- [x] 1.2 Add Qt coverage for method controls, silent replay including missing patterns, and modal replay prefill, accepted edits, and cancellation.

## 2. Implementation

- [x] 2.1 Add Halftone Screen and Custom Pattern to the render conversion.
- [x] 2.2 Connect the method controls and pattern picker to the Image Mode bridge and command recorder; implement silent execution and a modal context handler that pre-fills and returns edits.

## 3. Deferred checks and shipping

- [ ] 3.1 Run the render and Qt tests, OpenSpec validation, and repository gates when authorized.
- [ ] 3.2 When shipping, update ROADMAP.md and the PARITY.md assessment with the implementation commit.
