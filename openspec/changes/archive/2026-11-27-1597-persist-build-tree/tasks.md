# Tasks

## 1. Cache helper and its checks

- [x] 1.1 Add helper self-tests for missing generated output, missing Qt Test executable, changed generated digest, identity mismatch, build-input identity invalidation, configure-policy rejection, source timestamp normalization, and Ninja edge parsing. Verify that each test names the behavior it protects and is wired into `--self-test`.
- [x] 1.2 Implement manifest creation, restore validation, stale rejection, source and directory timestamp normalization, and build-log measurement using only the Python standard library. Verify by source inspection and the CI helper self-test step.

## 2. CI cache path

- [x] 2.1 Add manual forced-cold input and change-class output without changing existing job filters. Verify that pull requests and pushes still classify changes and manual runs can bypass restore.
- [x] 2.2 Compute the toolchain and build-input fingerprint, restore `build/` by stable prefix, and reject stale trees before configure. Verify that a restore failure removes `build/` and records `stale-rejected`.
- [x] 2.3 Keep the no-PCH, GPU-enabled configure, place FetchContent under `build/fetchcontent`, time Build, and record Ninja edges plus sccache statistics. Verify that the configure flags and app feature remain unchanged.
- [x] 2.4 Capture CTest registration, pass it to the reporter, and preserve one JUnit path per entry. Verify that a missing report remains a reporting failure.
- [x] 2.5 Create and save the manifest only after successful tests, save only on `master`, remove stale test reports before save, and publish size, restore, validation, upload, hit-rate, edge-count, and Build-duration measurements. Verify the save condition and JSON artifact wiring by inspection.

## 3. Review and CI evidence

- [x] 3.1 Review the actual diff for cache correctness, quota and restore cost, default-branch save policy, no-PCH and GPU preservation, complete Qt Test reporting, and a single build writer. Verify that every legitimate finding is fixed in the tree.
- [ ] 3.2 Run the first successful master cold save and record quota outcome, archive size, restore time, upload time, Build duration, Ninja edges, and sccache hit rate. Verify that `qt-build-cache-metrics.json` and the job summary contain the measurements.
- [ ] 3.3 Run warm no-op, Rust, shell-header, bridge-file, Qt Test, forced-cold, stale-rejection, and key-invalidation CI cases. Verify that only affected outputs rebuild and that incompatible identities miss.
- [ ] 3.4 Decide whether the cache pays against the shared Actions quota, then update or close #1056 with the broader measurement. Verify that no ten-minute claim is made without the full job data.
