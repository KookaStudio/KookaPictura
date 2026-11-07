# Verification

The pre-fix live control-server reproduction confirmed three symptoms: an already-rendered selection edge retained 90 dark outline pixels after entry, Ctrl-Z moved history back but left the center canvas pixel red, and a temporary 40×40 selection replaced the original 128×128 mask on Q exit. The fixed run clears the original outline, repaints the undone pixel white, and returns the original mask coverage rather than the temporary constraint.

## CPU brush timings

Fresh isolated application state, white RGB document, CPU compute, a centered half-width/half-height starting selection, Quick Mask, 20px brush at 100% hardness/opacity/flow. Twenty pointer moves spaced eight image pixels apart were timed through the existing control server. These are local reproduction timings, not CS6 oracle measurements.

| Canvas | Before median move | After median move | Before p95 | After p95 |
|---|---:|---:|---:|---:|
| 512×512 | 32.007 ms | 0.570 ms | 34.749 ms | 0.715 ms |
| 2048×2048 | 105.012 ms | 1.682 ms | 107.706 ms | 2.250 ms |

Initial fixes still measured about 74–82ms at 2048². Paint timing isolated the remaining delay to zoomed-out crop invalidation; patching the cached pyramid-level crop removed that cost. The regression checks assert bounded damage and cache reuse, rather than environment-dependent timing thresholds.

## Automated results

- Full CMake build with six compile jobs; no compilation until source/tests were written.
- Workspace fmt and clippy (`--workspace --all-targets -- -D warnings`) pass.
- `bash scripts/test-report.sh never`: **8,142 passed, 124 skipped, zero failed**. Rust: 5,122 passed/122 skipped. Application self-test: 507 passed. Qt: 2,513 passed/2 skipped, all 251 CTest entries pass.
- Actual canvas/keyboard checks cover cached ants, temporary brush/fill/filter clipping, exact visible undo/redo, consecutive strokes, mode-boundary history, reduced-zoom cache reuse, and geometry→Q→undo. Pure history checks cover metadata deltas/anchors/branches, named snapshots, purge, and shared coverage.
- Test-plan, file-size, repository, build-tree, and self-test budget guards pass. Existing budgets and golden assertions are preserved; this follow-up changes no golden baselines or dependencies.
- History memory accounting retains its existing document-plane/tile policy; selection metadata remains shared outside that accounting. No claim of independently measured CS6 parity is made.

The follow-up is synced and archived. Final strict OpenSpec validation passes all 390 specifications, and the zero-open archive guard passes.
