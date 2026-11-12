# Tasks

## 1. Tests named by the plan

- [x] 1.1 Qt: `tst_large_document::cancellingATransformEndsIt` (was `cancellingATransformResumesTheSession`).
- [x] 1.2 Qt: `tst_memory_governor::aSpillingTransformRunsAsAJob` expects the transform to end on Cancel.

## 2. App

- [x] 2.1 `commitFreeTransform` and `commitTransformControls` end the session after a cancelled job.

## 3. Close-out

- [x] 3.1 `openspec validate --all --strict`, `scripts/check-test-plan.py`, one build, the Qt suites.
- [x] 3.2 Archive the change in the PR.
