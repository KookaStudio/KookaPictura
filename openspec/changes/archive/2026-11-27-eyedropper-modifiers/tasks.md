# Tasks

## 1. Interaction coverage

- [x] 1.1 Write all eleven Qt Test cases named in `test-plan.md`, covering each scenario and its observable result.

## 2. Eyedropper gestures

- [x] 2.1 Implement foreground/background destination and fixed drag ownership; coverage is written in `altSamplesBackground` and `plainSamplesForeground`.
- [x] 2.2 Implement Shift sampler placement and four-sampler refusal, with Shift+Alt precedence; coverage is written in `shiftPlacesSampler`, `shiftSamplerLimit`, and `shiftAltPlacesSampler`.

## 3. Temporary sampling

- [x] 3.1 Share scoped, continuous foreground sampling with the six eligible painting tools and preserve selected tool, options, cursor and active stroke ownership, including across Alt release; allow temporary sampling in Quick Mask without changing its coverage; coverage is written in `temporarySampling`, `temporaryRestores`, `temporaryCancel`, `temporaryQuickMask`, and `altDuringStroke`.
- [x] 3.2 Preserve the established Alt actions of Clone Stamp, Healing Brush, Eraser and Mixer Brush; coverage is written in `reservedAltGestures`.

## 4. Integration

- [x] 4.1 Review and update ROADMAP.md and PARITY.md for the delivered coverage.
- [x] 4.2 Run the planned Qt Test cases and appropriate project gates in Arch x86-64 Docker.
