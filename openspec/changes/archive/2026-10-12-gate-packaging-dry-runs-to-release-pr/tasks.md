# Tasks

## 1. Gate the pull-request trigger on all three packaging workflows

- [x] 1.1 In `.github/workflows/windows.yml`, replace the `pull_request.paths`
  list (workflow/action/packaging/`crates/**`/build files) with the five
  Release-PR files (`Cargo.toml`, `CMakeLists.txt`, `CHANGELOG.md`,
  `version.txt`, `.release-please-manifest.json`) and add the job-level gate
  `if: github.event_name != 'pull_request' || github.head_ref ==
  'release-please--branches--master'`. Keep `workflow_call` and
  `workflow_dispatch` unchanged.
- [x] 1.2 Apply the same paths list and job gate to
  `.github/workflows/linux-packages.yml`.
- [x] 1.3 Apply the same paths list and job gate to
  `.github/workflows/appimage.yml`.

## 2. Cancel superseded pull-request runs

- [x] 2.1 Add to each of the three workflows:

  ```yaml
  concurrency:
    group: ${{ github.workflow }}-${{ github.ref }}
    cancel-in-progress: ${{ github.event_name == 'pull_request' }}
  ```

  so Release-PR iteration cancels the previous run and tag builds never are
  cancelled.

## 3. Add sccache to both bundle actions

- [x] 3.1 In `.github/actions/windows-bundle/action.yml` add
  `mozilla-actions/sccache-action@v0.0.11` after the MSVC setup step and add
  `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` to the CMake configure line.
- [x] 3.2 In `.github/actions/linux-bundle/action.yml` add the same sccache
  action before the build step and the same CMake flag.
- [x] 3.3 In all three workflows set job env `RUSTC_WRAPPER: sccache`,
  `SCCACHE_GHA_ENABLED: "true"`, and `CARGO_INCREMENTAL: "0"` alongside the
  existing `TAG`/`VERSION` env (the pattern from `ci.yml`). No new dependency:
  `mozilla-actions/sccache-action` is already used there.

## 4. Update workflow headers and DEVELOPING.md

- [x] 4.1 Rewrite the header comment of each of the three workflows: the dry
  run now happens on the release-please Release PR, not on any PR touching the
  packaging.
- [x] 4.2 Rewrite the two `DEVELOPING.md` "Dry run" bullets (Linux packages
  ~line 385, Windows packages ~line 413) to describe the Release-PR trigger
  and note the dispatch escape hatch's tag-checkout limit. `DEVELOPING.md` is
  outside `docs/`, so no `TASK-ALLOWS-DOCS` marker is required.

## 5. Validation and verification

- [x] 5.1 `openspec validate --all --strict` passes.
- [x] 5.2 Push the branch and confirm none of the three workflows starts a
  runner for it (either not scheduled, or the job reports skipped because the
  head branch is not `release-please--branches--master`).
- [x] 5.3 After merge, push to `master` so the Release PR syncs, then confirm
  all three workflows run as dry runs on the Release PR and keep their
  artifacts.
- [x] 5.4 Compare the second Release-PR run's wall time against the first
  (sccache cold vs warm): windows should drop from ~14 min toward ~6–8 min,
  linux-packages/appimage from ~13 min toward ~6–7 min.