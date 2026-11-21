# Tasks

## 1. Version and About

- [x] 1.1 Add `tst_update_preferences::versionFlagWorksWithoutDisplay`. Pass
  the built `pictura` target path through CMake and assert the process behavior
  in the test plan, including stdout, stderr, exit status, state and requests.
- [x] 1.2 Extend `tst_update_preferences::aboutShowsVersionAndMethod` to
  inspect the dialog opened through the About command. Add
  `aboutShowsRuntimeDetails` with a live adapter and driver comparison against
  the renderer bridge. Add `aboutSeparatesCpuRendererFromAdapter` and
  `aboutDisplaysUnavailableAdapterFields` for the fixture cases in the test plan.
- [x] 1.3 Extend `tst_system_info::dialogCopiesReport` to assert that the
  clipboard includes the populated, detected package method.
- [x] 1.4 Extend the Windows packaging smoke step to pipe the staged
  `pictura.exe --version` output in PowerShell. Check for exactly one version
  line matching the checkout's `version.txt` and exit status 0.
- [x] 1.5 Handle standalone `--version` before the GPU-probe path and Qt
  application initialization. Check that order in `main()` and verify the
  exit status and version line with `versionFlagWorksWithoutDisplay`.
- [x] 1.6 Extend the About handler and formatter with the configured build
  type, runtime Qt version, and cached adapter details. Adjust the existing
  `tst_gpu_backend::aboutTextNamesBackend` case if the formatter signature
  changes. Verify the new About checks and the existing renderer-label check.
- [x] 1.7 Update the `ROADMAP.md` entry for #649 to mark the version flag and
  About diagnostics shipped and credit #1146 for the already-shipped stable
  checker. Verify that the entry accurately describes the released surfaces.

## 2. Integration verification

- [x] 2.1 After edits are complete, build with `scripts/with-build-lock.sh`
  using the repo's CPU budget. Run `tst_update_preferences`,
  `tst_gpu_backend`, and `tst_system_info` through wrapped CTest. Run
  `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`
  while the change is active.

## 3. Before shipping

Once the tasks and pre-archive checks pass, archive the change in the shipping
pull request. Verify the archived tree with `bash scripts/check-openspec-archived.sh`.
The Windows packaging workflow runs the staged-binary smoke check on the Release
PR. For earlier Windows verification, use the same bundling and smoke commands
from the feature checkout.
