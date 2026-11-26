# Design

## Context

See proposal.md, Why. The current `test` job builds `pictura_app` twice for the
oracle checker. The `changes` job uses `dorny/paths-filter` once, derives only
`cpp_only` and `render`, and lets a failed fetch skip dependent jobs. The Qt job
already runs Corrosion with the `dev-app` profile, producing the normal library
artifact the checker needs.

## Goals / Non-Goals

Goals:

- Remove both oracle-only Cargo builds without moving compile work onto the
  `test` critical path.
- Make classification retry once and make every failure mode run required
  coverage.
- Keep bucket logic testable with the Python standard library.
- Preserve the existing test commands except for the duplicated Windows CPU
  tests.

Non-Goals:

- Replace `dorny/paths-filter` or change how it resolves PR and push bases.
- Change packaging workflows, test selection, golden baselines, or product code.
- Add a dependency or a new CI service.

## Decisions

### Reuse Corrosion JSON instead of a nextest report

Nextest's test-profile artifacts are not a valid input for this audit. They are
test builds, and application tests enable the `test-oracle` development feature.
The Corrosion `dev-app` build is a normal library build with the feature off and
already runs in `qt-headless`.

Add an opt-in CMake flag that appends `--message-format=json` to the existing
Corrosion Cargo command. CI sets the flag, tees the mixed CMake/Ninja/Cargo log
to a file, and runs the checker after the build. The checker reads JSON object
lines from the mixed log and keeps its existing uniqueness, profile, feature,
and artifact checks. A malformed JSON-looking line fails the audit.

Alternative considered: emit a report from nextest. Rejected because its
compiler artifacts do not meet the normal-library contract. The issue's
fallback, dropping only the `dev-app` build, is unnecessary because Corrosion
supports the required flag.

### Retry the existing action, then classify in Python

Keep the SHA-pinned `dorny/paths-filter` for base resolution. Mark the first
attempt `continue-on-error` so a successful retry can restore a green classifier
job. Run the same step once more only when the first attempt's outcome is
`failure`. Request its file list as JSON so filenames with spaces stay intact. A
small standard-library script classifies the list and has a `--self-check` mode.

Alternative considered: a retry action or a hand-written GitHub API client.
Rejected because it adds a dependency or reimplements the base resolution that
the existing action already owns.

### Fail closed by publishing run-all outputs and failing

When both fetches fail, or the file list cannot be parsed, the classifier writes
`rust=true`, `cpp=true`, `render=true`, and `docs=false` before exiting non-zero.
Every gated job uses `always()` and skips only on an exact `false`. Missing
outputs therefore run the job. The classifier remains red, so the outage is
visible, but it cannot silently remove Rust coverage.

Alternative considered: succeed with run-all outputs. Rejected because it hides
a repeated classifier outage.

### Use overlapping buckets to preserve Qt coverage

`cpp` means the Qt application build is affected, not only that a hand-written
C++ file changed. Every non-docs change sets it because the application links
the Rust staticlib. `rust` is false only for a docs-only or pure
`crates/pictura-app/cpp/` change. `render` uses the existing precise allowlist
plus CI workflow changes. This keeps `qt-headless` on Rust changes while
removing the 30-minute GPU job from MCP and script-only changes.

### Keep Windows work that is unique to Windows

The Windows job keeps both Clippy steps and the DX12 WARP/FXC smoke. Remove the
default library test and the `backend_pin`/`backend_cpu` commands. Linux already
runs those tests through workspace nextest and the dedicated renderer nextest
run.

## Risks / Trade-offs

- Mixed CMake output could break JSON parsing. The checker accepts non-JSON
  build lines, rejects malformed JSON-looking lines, and self-checks both cases.
- A cached CMake tree might not rerun Cargo and produce no fresh JSON. CI starts
  from a fresh build tree; a missing report fails the headless job.
- Both diff fetches could fail. The classifier fails red and publishes run-all
  outputs; `always()` conditions keep the build jobs running.
- Concurrent `ci.yml` work may conflict during integration. Rebase onto the
  current `origin/master`, then re-check the outputs, retry step, and every job
  condition together.
- The direct release-profile oracle probe disappears. The normal feature gating
  remains, and Release PR packaging still builds the shipped configuration.

## Migration Plan

Implement the CMake flag, checker parsing, classifier, and workflow gates in one
change. The first CI run exercises the new path. Rollback is a single revert;
no data or artifact migration is needed.

## Open Questions

None.
