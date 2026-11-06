# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Engine

- [x] 2.1 `PeakAlloc` in `pictura-testkit`
- [x] 2.2 Install it in the `pictura-app` unit-test binary; the `tests_profiles/memory` guards and full-size reports
- [x] 2.3 A nextest test group that runs the memory guards two at a time

## 3. App

- [x] 3.1 The `free_transform` op on `layer_op`

## 4. Scripts

- [x] 4.1 `gen-large-fixtures.py`
- [x] 4.2 `memory_scenarios.py` and `profile-tools.py --peak`, `--memory-max`, outcomes, compare, `--self-check`
- [x] 4.3 `check-test-plan.py` resolves script references; `verify-full.sh` runs the self-checks
- [x] 4.4 Refuse a `--memory-max` the machine cannot spare without `--force`

## 5. Baseline

- [x] 5.1 Record the 4250² heap baselines in the guards
- [x] 5.2 Run M1–M13 at 17000² under `4G` and commit `docs/dev/benchmarks/memory-baseline.json`
