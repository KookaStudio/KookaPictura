# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per spec scenario

## 2. Tests

- [x] 2.1 New `tst_eyedropper` suite: `eyedropperDragSamplesContinuously`
- [x] 2.2 `tst_eyedropper::pickerSwatchEyedropperSamplesTheCanvas`
- [x] 2.3 Register `tst_eyedropper` in `crates/pictura-app/cpp/tests/CMakeLists.txt`

## 3. Controller drag sampling

- [x] 3.1 `tools.{h,cpp}`: add `canvasSampling_`; sample on move while the left
  button is held; clear on release and when the sampler is replaced

## 4. Eyedropper tool drag

- [x] 4.1 `tool_eyedropper.cpp`: sample on move while the press gesture is live

## 5. Picker default sampler

- [x] 5.1 `color_picker_dialog.{h,cpp}`: `setDefaultCanvasSampler`; `getColor`
  resolves the default when the caller passes none; `sampleCanvas` leaves an
  already-armed sampler in place
- [x] 5.2 `frame_build.cpp`: install the default sampler (`QPointer`-guarded,
  `activeView()->sample_argb`)

## 6. Roadmap

- [x] 6.1 `ROADMAP.md`: tick the continuous eyedropper drag (#1040)

## 7. Verify

- [x] 7.1 Build, then run the touched suites, `openspec validate --all --strict`,
  and `bash scripts/verify-fast.sh`
