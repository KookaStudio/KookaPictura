# Tasks

## 1. Scenario coverage

- [x] 1.1 Write Qt tests for each test-plan row, including a focused tablet-input test, before completing its corresponding implementation.

## 2. Magnetic Pen

- [x] 2.1 Implement and persist the controls and bracket shortcuts; cover with `magneticOptions` and `magneticPressure`.
- [x] 2.2 Implement edge-following, fastening, cancellation, Alt gestures and path completion; cover with `magneticTrace`, `magneticFrequency`, `magneticDepth`, `magneticSwitch`, `magneticCancel`, `magneticOpen`, `magneticClose`, and `magneticAlt`.
- [x] 2.3 Route completed traces through all output modes and history; cover with `magneticModes`, `magneticQuickMask`, and `magneticActivePaths`.
- [x] 2.4 Update the feature trackers and archive the completed change.

## 3. Deferred verification

- [x] 3.1 Run the planned Qt tests and scoped repository gates. Mac Docker verified source `7173211dae9a0d4449fafe550b9bab5c8b0a40bd`; the two Qt suites passed in 4.02 seconds and the packaged headless self-test reported 466 passed, 0 failed, and 1 skipped.
