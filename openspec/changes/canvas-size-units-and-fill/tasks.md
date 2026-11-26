## 1. Regression coverage

- [x] 1.1 Write Qt cases for Picas, absolute and relative Columns, and extension
  controls and resulting pixels with and without a Background.
- [x] 1.2 Cover retained off-canvas content and a layerless merged-composite
  document, including the resize transaction's fill decision.
- [x] 1.3 Cover large unit restatements and the shared Background predicate for
  a flagged non-pixel layer.

## 2. Implementation

- [x] 2.1 Add the new unit choices and convert through document PPI, point
  definition, and saved column geometry.
- [x] 2.2 Gate both extension controls and the resize transaction's fill on the
  original top-level Background, including direct and Actions callers.
- [ ] 2.3 Run the applicable builds, tests, and checks when authorized.
