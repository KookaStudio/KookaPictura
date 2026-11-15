# Test Plan

Covers every scenario in the `meta/flatpak-release-build` delta. The behavior is
CI runner/filesystem compatibility that no Rust or Qt test can exercise, so every
row reads `manual:` and names the dry run, log line, or cache operation.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `fp_runtime_update` | Rerun the Release PR's `flatpak` job until `Set up job` reports `Cloud: AWS`, then read the build step | The runtime update retires the image's checkout and the build continues with no `Invalid cross-device link` line | manual: GitHub Actions dry run, `Set up job` cloud line and build log |
| `fp_versions_named` | Read the "Name the Flatpak tool and runtime versions" step in the job log | The log names the `flatpak` and `flatpak-builder` versions, the installed runtime refs and commits, and the kernel | manual: GitHub Actions dry-run log |
| `fp_dry_run_cache_paths` | `gh cache delete flatpak-builder-x86_64-<key>`, rerun the job, then rerun it again with the cache restored | Both runs build the bundle, keep it as a workflow artifact, and do not fail at the runtime update | manual: GitHub Actions dry run plus `gh cache delete` |

The 0.4.0 tag build is the first tagged build on the fixed workflow and runs the
same job; it is confirmed on the release after this change merges.
