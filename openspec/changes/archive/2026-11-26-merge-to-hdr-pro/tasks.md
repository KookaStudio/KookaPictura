## 1. Regression coverage

- [x] 1.1 Add the engine checks named in the test plan for analytical radiance, response replay and rejection, alignment ambiguity, ghost base, per-channel saturation, alpha, native PSD/PSB round trips, 16-bit conversion precision, and memory overflow.
- [x] 1.2 Add the Qt modal-harness checks named in the test plan for source eligibility and depth refusal, automatic EV100, preview consistency, float output, failures, and cancellation.

## 2. Implement

- [x] 2.1 Implement the deterministic parametric sRGB inverse and exponent fit, exposure-normalized fusion, alignment failure, selected ghost base, and saturation/alpha fallback.
- [x] 2.2 Wire the source picker, automatic EV100 estimation, temporary source stack, float tone preview, settings exchange, budget reservation, and publication; keep inputs untouched.
- [x] 2.3 Update coverage trackers and record #558's remaining PSD/PSB/TIFF preview white-point persistence gap.

## 3. Follow-up

- [ ] 3.1 Compile, run the planned automated tests, and validate the OpenSpec change.
- [ ] 3.2 Track the deferred Debevec oracle comparison until a reference and agreed tolerance are available; do not claim verified parity.
