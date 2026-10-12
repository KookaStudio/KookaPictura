# Proposal

## Why

The three packaging workflows (`windows.yml`, `appimage.yml`,
`linux-packages.yml`) run their full build as a pull-request dry run. That
costs more than it returns: `windows.yml` fires on every PR touching
`crates/**` (~14 min per push), and `linux-packages.yml` matches `packaging/**`
so PRs that only add `packaging/windows/**` re-run a ~13 min `.deb`/`.rpm`
build for nothing (ten wasted runs on the Windows-packaging branch alone).
None of these dry runs gate merge. They are informational, so the signal they
add on feature PRs does not justify the runner time. One policy replaces three
path lists: release artifacts are validated on the release-please Release PR,
the last pull request before the tag.

## What Changes

- Gate the `pull_request` trigger of `windows.yml`, `appimage.yml`, and
  `linux-packages.yml` to the release-please Release PR
  (`github.head_ref == 'release-please--branches--master'`), with `paths`
  narrowed to the five files the Release PR bumps (`Cargo.toml`,
  `CMakeLists.txt`, `CHANGELOG.md`, `version.txt`,
  `.release-please-manifest.json`). `workflow_call` and `workflow_dispatch`
  are unchanged: the tag build and the manual re-run still work as before.
- Add a `concurrency` group to each of the three workflows that cancels
  superseded pull-request runs (none has one today); release builds are never
  cancelled.
- Add `sccache` to the two bundle composite actions
  (`.github/actions/windows-bundle`, `.github/actions/linux-bundle`), following
  the pattern already proven in `ci.yml`: `RUSTC_WRAPPER` +
  `SCCACHE_GHA_ENABLED` env and `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` on the
  CMake configure line, so the runs that remain compile only what changed
  instead of a cold cargo + C++ build every time.
- Update `DEVELOPING.md` and the three workflow header comments to describe
  the new trigger.
- Accepted tradeoff: a feature PR that changes packaging (for example
  `packaging/build-packages.sh`) is no longer validated pre-merge; the next
  Release PR sync after the merge is where the dry run catches it. The
  `workflow_dispatch` escape hatch stays, with the known limit that it runs the
  dispatched branch's workflow file against the tag's checked-out tree.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `meta/windows-release-build`: the dry-run requirement previously said any
  pull request touching the Windows build runs it; it now runs only on the
  Release PR. The documentation requirement's trigger list changes to match.
  The `appimage.yml` and `linux-packages.yml` dry runs have no capability spec
  (only `DEVELOPING.md` documents them), so they need no delta.

## Impact

- **CI**: three workflow files gain a job-level gate, narrower `pull_request`
  paths, and a `concurrency` group; two composite actions gain one `sccache`
  step each; no new third-party actions beyond `mozilla-actions/sccache-action`
  (already used in `ci.yml`).
- **Specs**: `meta/windows-release-build` replaces the old dry-run requirement
  (REMOVED + ADDED, because its "packaging pull request dry-runs" scenario no
  longer applies) and amends the documentation requirement's trigger list;
  `meta/release-please` is untouched (the tag remains the packaging trigger).
- **Docs**: `DEVELOPING.md` dry-run bullets for Linux and Windows packaging.
- **Dependencies / app code**: none.