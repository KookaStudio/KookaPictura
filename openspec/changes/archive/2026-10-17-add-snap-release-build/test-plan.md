# Test Plan

Covers every scenario in the `meta/snap-release-build` delta. The capability is
CI/packaging behavior, which `scripts/check-test-plan.py` cannot name —
its coverage tokens are `tst_<suite>::<case>` (Qt Test) and `<crate>::<fn>`
(Rust) — so every row reads `manual:` naming the CI check or the observable
artifact. There is no engine or GUI logic to unit-test; the workflow and
`snapcraft.yaml` are exercised by the dry run itself.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `A release is cut` | Merge the release-please Release PR so tag `vX.Y.Z` is created and the `snap` job runs | The release gains `kooka-pictura_X.Y.Z_amd64.snap` as an asset | manual: release-asset check on the GitHub Release |
| `The build is re-run for an existing tag` | `gh workflow run snap.yml -f tag=vX.Y.Z --ref <branch>` | The `.snap` is rebuilt and re-uploaded, replacing the existing asset (`--clobber`) | manual: dispatch run re-uploads the asset |
| `The application compiles under the snap base` | Inspect the `snap` job log for this PR's dry run | The build completes against the `kde-neon-6` Qt SDK and the snap links the extension's Qt, not a bundled copy (`ldd`/`snapcraft` output) | manual: snap workflow build log |
| `The self-test passes under strict confinement` | The smoke-check step installs the built snap and runs `pictura --headless --self-test` | Exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0` | manual: snap workflow smoke-check step |
| `The snap declares its plugs` | Run `snapcraft expand-extensions` (or inspect the built snap) and check the app's plugs | `home`, `removable-media`, and `cups` are declared alongside the extension's desktop/GPU/network plugs | manual: `snapcraft expand-extensions` output review |
| `release-please triggers the build` | Confirm the `snap` job in `release-please.yml` is called with the tag on a release | The snap workflow runs with `tag: vX.Y.Z` | manual: `release-please.yml` job wiring review |
| `The Release PR dry-runs` | Open/update the release-please Release PR | The workflow builds the snap and keeps it as an artifact; nothing is uploaded to a release | manual: Release-PR dry-run artifacts vs the release |
| `A feature pull request does not run it` | Open a feature PR whose head branch is not `release-please--branches--master` | The snap job is skipped and no snap runner starts | manual: workflow run list for the feature PR |
| `A maintainer reads the docs` | Read the "Snap packages" section of `DEVELOPING.md` | Artifact, `strict` confinement, release-asset publish + store non-goal, dry-run trigger, and the re-run command are present | manual: docs review |
