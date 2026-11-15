# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in
  `specs/meta/flatpak-release-build/spec.md`, each row reading `manual:` and
  naming the dry run, log line, or cache operation; verify
  `python3 scripts/check-test-plan.py` passes.

## 2. Workflow

- [x] 2.1 Add `volumes: - /var/lib/flatpak` to the `flatpak` job container in
  `.github/workflows/flatpak.yml`, with a comment naming the overlayfs rename
  cause, and annotate the image digest with `# kde-6.11`; verify the workflow
  parses and the diff touches only the `flatpak` job.
- [x] 2.2 Add a pre-build step that prints `flatpak --version`,
  `flatpak-builder --version`, `flatpak list --system --runtime
  --columns=ref,active`, and `uname -r`; verify it precedes "Build the
  bundle" in the parsed workflow.

## 3. Verification

- [ ] 3.1 After the change is on the Release PR, rerun its `flatpak` job until
  `Set up job` reports `Cloud: AWS`; verify the build passes with no `Invalid
  cross-device link` line and the diagnostic step names the tools, runtimes,
  and kernel.
- [ ] 3.2 Delete the action cache (`gh cache delete
  flatpak-builder-x86_64-<key>`) and rerun the job; verify it passes, then
  rerun once more with the restored cache and verify that pass too.
- [x] 3.3 Archive the change in the shipping pull request
  (`openspec archive fix-flatpak-runtime-update --yes`) and verify
  `openspec validate --all --strict` and `bash scripts/check-openspec-archived.sh`
  pass.

The 0.4.0 release is the first tagged build on the fixed workflow. It runs the
same `flatpak` job as the verified dry run; confirm it on the release rather
than dispatching `v0.3.0`, whose upload job would rewrite that release's
existing asset.
