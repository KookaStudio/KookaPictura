## 1. Curve editor

- [x] 1.1 Share exact point controls, black and white sliders, and graph display controls.
- [x] 1.2 Implement dense pencil drawing, smoothing and preview evaluation and add bounded private trailer parsing, sparse fallback, exact source-content preservation and sibling-channel preservation.
- [x] 1.3 Extend the model and bridge for five CMYK curves and adapt channel editing, graph units and RGB-backed CPU/native rendering for Gray, CMYK and Lab, with GPU fallback for non-RGB modes.

## 2. Document integration

- [x] 2.1 Wire targeted gestures and endpoint/gray sampling into dialog preview and layer history.
- [x] 2.2 Reuse the fixed histogram Auto for the selected RGB or Gray channel, retain no-op handling and disable it for CMYK and Lab.

## 3. Coverage

- [x] 3.1 Add Qt and Rust regressions from the test plan.
- [ ] 3.2 Build, run tests, and validate the change in the follow-up verification phase.
