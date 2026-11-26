# Proposal

## Why

The PR critical path spends about 142 seconds on two application builds that
exist only to feed the view-oracle audit. A transient changed-file fetch can
also skip Rust coverage without a useful signal, while the GPU job uses a
coarse C++-only bucket and the Windows job repeats CPU tests already run on
Linux.

## What Changes

- Reuse the `dev-app` Cargo JSON emitted by the existing Corrosion build in
  `qt-headless`. Remove both throwaway application builds from `test` and teach
  the oracle checker to read Cargo JSON records from the mixed CMake log.
- Replace `cpp_only` with `rust`, `cpp`, `render`, and `docs` outputs. Retry the
  changed-file fetch once. If classification still fails, publish run-all
  outputs, fail the classifier, and let gated jobs run through `always()`.
- Gate `lint` and `test` on `rust`, `qt-headless` on `cpp`, and `gpu-parity` and
  `windows-render` on the precise `render` bucket.
- Keep both Windows Clippy steps and the DX12 WARP/FXC smoke. Remove the
  duplicated Windows CPU test commands.
- Keep `lint` and `test` separate, keep the standalone
  `cargo nextest run -p pictura-render`, and leave Release PR packaging dry runs
  unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/verification-harness`: CI publishes explicit path buckets,
  retries classification once, and fails closed without skipping required
  coverage.
- `verification/gpu-parity-ci`: GPU and Windows render jobs use the precise
  render bucket, and Windows runs only its unique Clippy and DX12 smoke checks.
- `verification/view-pyramid-oracle`: CI audits oracle exclusion from the
  existing Corrosion `dev-app` build instead of starting extra Cargo builds.

## Impact

Affected files are `.github/workflows/ci.yml`, `CMakeLists.txt`,
`scripts/check-view-oracle.py`, a new standard-library path classifier, and the
change specs/test plan. No dependency, product API, test baseline, or packaging
workflow changes.
