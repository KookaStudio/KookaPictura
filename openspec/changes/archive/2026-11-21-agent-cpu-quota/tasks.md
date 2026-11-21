## 1. Test plan

- [x] 1.1 Write the fixture checks for unit generation, scope commands, quota validation, cache isolation and opt-in lock stability.
- [x] 1.2 Add an actual queue regression to the build-lock check.

## 2. Implementation

- [x] 2.1 Implement the opt-in launcher and enrollment check with a dedicated sccache endpoint.
- [x] 2.2 Keep legacy sccache affinity operations out of enrolled runs and away from enrolled servers.
- [x] 2.3 Resolve the optional build lock independently of caller-specific temporary directories.
- [x] 2.4 Wire script checks into verification and document setup, harness enrollment, live checks, and removal.

## 3. Verification

- [x] 3.1 Run the affected script gates and validate the change and its test plan.
- [x] 3.2 Exercise a temporary low-quota slice on the available user manager and record the remaining real-session checks for review.
