# Tasks

## 1. Scenario checks

- [ ] 1.1 Write `pictura_render::content_aware_amount_endpoints_and_midpoint`, `pictura_render::content_aware_protect_biases_seams`, `pictura_render::content_aware_axes_and_enlargement`, `pictura_render::content_aware_native_components` and `pictura_render::content_aware_invalid_is_atomic`; confirm each asserts its test-plan outcome.
- [ ] 1.2 Write `tst_content_aware_scale::sessionGeometry`, `refusesTargets`, `selectionLimitsTarget`, `exclusiveSession`, `protectControls`, `supersededPreview`, `cancelRestoresState`, and `commitUndoRedo`; register the suite and confirm each asserts its test-plan outcome.

## 2. Raster operation

- [ ] 2.1 Implement deterministic forward-energy seam removal and insertion in a separate module, including aligned planes, native depths including widening new unconverted high-depth layers, two axes, small images, checked budgets and fallible allocation; verify the axes, native and invalid tests.
- [ ] 2.2 Implement alpha-channel protection and premultiplied Amount blending against ordinary scaling; verify protection and Amount tests.

## 3. Session and integration

- [ ] 3.1 Add the interactive session and options for geometry, Amount and Protect, with eligible-target, mask and missing converted-source refusal, and 8-bit selection handling. Use the exclusive session slot, transform locks and pending-switch resolution; coordinate shared registrations with #527. Verify session, refusal, selection, exclusive and control tests.
- [ ] 3.2 Stage original-source previews in cancellable generation-checked jobs, enforce the specified budgets and disable commit until the current result completes. Implement commit, cancel and lossless history; verify superseded preview, failure, cancel and history tests.
- [ ] 3.3 Review ROADMAP.md and PARITY.md, update feature coverage and issue links together, and record the behavioral approximation and eligibility limits without claiming Adobe parity.

## 4. Integration

- [ ] 4.1 After all code is complete, build once within the CPU budget, fix any errors and rebuild only after changes; run the scenario checks and project gates as authorized.
- [ ] 4.2 Review and archive this change after implementation, including any manual test-plan rows in the PR checklist.
