# Design

## Release PR as the only pull-request dry run

GitHub has no head-branch filter on `pull_request`, so gating is two-part:

1. `on.pull_request.paths` narrows which PRs even start the workflow. It lists
   the five files release-please bumps in every Release PR (verified against
   the v0.2.0 release commit: `Cargo.toml`, `CMakeLists.txt`, `CHANGELOG.md`,
   `version.txt`, `.release-please-manifest.json`). Feature PRs touching other
   files never schedule the workflow; feature PRs that do touch one of the
   five start it but hit step 2.
2. A job-level `if` accepts only the Release PR head:

   ```yaml
   if: github.event_name != 'pull_request' || github.head_ref == 'release-please--branches--master'
   ```

   `github.head_ref` is empty outside pull requests, but the first operand is
   true for `workflow_call` (event inherited from the `release-please.yml`
   caller: `push`) and for `workflow_dispatch`, so the tag build and manual
   re-run pass the gate untouched. Paths filters do not apply to those two
   triggers.

Merge does not double-fire: `pull_request` defaults to
`opened/synchronize/reopened`; the merge itself is `closed`.

## Why not keep narrow per-workflow paths

Narrowing `linux-packages.yml` from `packaging/**` to its two real inputs
(`packaging/build-packages.sh`, `packaging/pictura.desktop`) would fix the
observed waste without a policy change, and `appimage.yml` already lists
`packaging/pictura.desktop` explicitly. Rejected because it leaves three
workflows with three ad-hoc path lists whose pre-merge signal nobody gates on,
while `windows.yml` still fires on every `crates/**` PR. That is the volume
problem the Release-PR gate solves uniformly.

## Concurrency

Each workflow gains:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

Superseded Release-PR pushes cancel their in-flight run (the pattern `ci.yml`
already uses unconditionally); tag builds (`refs/tags/...` or the caller's
`push` ref) are never cancelled because `cancel-in-progress` is false for them.

## sccache

Both bundle actions build cold today: `Swatinem/rust-cache` runs with
`cache-targets: "false"` (registry only) and neither sets a compiler wrapper,
so cargo recompiles all dependencies (7m00s Windows, 5m44s Linux) and the C++
shell recompiles (~4 min Windows) on every run. The fix reuses `ci.yml`'s
proven setup unchanged: `mozilla-actions/sccache-action@v0.0.11`, job env
`RUSTC_WRAPPER: sccache` / `SCCACHE_GHA_ENABLED: "true"` /
`CARGO_INCREMENTAL: "0"`, and `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` on the
configure line (no linker change). The step sits after toolchain/MSVC/Qt
setup and before the build; the first run stays cold, later runs share the
cache with the tag build because it is keyed per repository, not per workflow.

Expected wall time once warm: windows ~6–8 min (from 14m19s measured),
linux-packages / appimage ~6–7 min (from ~13 min measured).

## Dispatch escape hatch

`gh workflow run <wf> -f tag=vX.Y.Z --ref <branch>` still works for
rebuilding an existing tag. It reads the workflow file from `<branch>` but
`actions/checkout` checks out the tag, so the composite action and packaging
files come from the tag's tree. Workflow-only edits are testable; bundle or
packaging edits are not. Those validate on the next Release PR sync instead.
Documented as the known limit of the escape hatch.