# kooka-pictura — core

Documentation-first reimplementation of **Adobe Photoshop CS6 (v13)** in **Rust + Qt6**, Linux.
`docs/` is the long-form behavioral contract; implementation lives in `crates/`.

## Two halves

- **Engine crates** (`crates/pictura-{core,codec,color,adjust,filters,ops,paint,select,render,testkit}`):
  spec math, **Qt-free**, testable headless. `pictura-core` has no deps.
- **App** (`crates/pictura-app`): the only Qt crate. Rust `staticlib` (cxx-qt `PictureView` QObject)
  linked into the C++/Qt `pictura` binary by CMake. See `mem:app/architecture`.

## Source map

```
docs/          spec corpus + docs/dev/ working notes (read-only unless the task allows docs)
openspec/      change proposals (changes/) + canonical capability specs (specs/)
crates/        engine crates + the app (see crate table in docs/dev/STATE.md)
scripts/       verification gates + Python oracle tools
```

`docs/dev/STATE.md` is the resume anchor ("where things are") — read it first for current work,
but treat its counts/facts as a snapshot, not durable truth. `docs/dev/testing-conventions.md` is
the source of truth for how every test layer is built and reports. `AGENTS.md` is the agent guide.

## Project-wide invariants

- **Specs are the contract.** Do not change `docs/` unless the task says so; a docs change needs
  `TASK-ALLOWS-DOCS` in the commit message (or `TASK_ALLOWS_DOCS=1`) or `scripts/guard.sh` fails.
- **Determinism.** Golden/oracle comparisons run on CPU; GPU is an accelerator, never the oracle.
  Fix seeds for anything random.
- **Parity only where an oracle exists.** Adobe's closed algorithms are approximated and marked as
  such; never claim verified parity without a test that proves it.
- **Milestones (`mNN`) live only in comments, docs, and specs** — never in identifiers, string
  literals, or test names (`scripts/check-milestone-names.py`, run by `guard.sh`).
- **No new dependencies** without stating why; prefer std and already-present crates.
- **File size cap** and other code rules: `mem:conventions`.

## Related memories

- Build/run/gate commands: `mem:suggested_commands`.
- Toolchain and version pins: `mem:tech_stack`.
- Code style, oracle/test rules, file-size caps: `mem:conventions`.
- Definition-of-done and the OpenSpec workflow: `mem:task_completion`.
- The Qt/cxx-qt app shell and its hand-rolled self-test: `mem:app/architecture`.

## Banded CPU preparation

`CompositePlan::build_banded` resolves ordered sibling nodes on the existing Rayon pool, recursively through visible groups, before any band replays the plan. Ordinary whole-canvas, region, native-depth, and render-session builders keep sequential layer resolution. Plans retain effect-pass Arcs through replay even if the cache evicts them. Source-cache worker non-waiting semantics and keys are unchanged. Feather-cache misses likewise build outside locks and publish resolved Arc<Feathered> planes, permitting duplicate cold work without blocking nested Rayon workers; scalar lookup releases its thread-local borrow before construction. Test-only per-call preparation selection and document-scoped observers live with `tests/effect_preparation`; they do not consult `FORCE_SERIAL`. See the archived `parallel-effect-passes` change (#1179).

The #1179 latency follow-up incorporates PR #1221's exact Gaussian/bevel kernels, resolves matte mask gates once, quantizes through bulk plane slices, emits constant-alpha rows in parallel, and replays common effect modes through row kernels. The u8 Gaussian vertical pass uses 16×128 cache-local tiles with unchanged per-pixel tap order. Scalar tests cover tile edges, mask refinements, and all replay components/coverage. Use the ignored production integration test `effect_cold_latency` for latency acceptance: unit binaries increment a global mask-sample atomic per pixel and can substantially overstate production render time. Three fresh production processes pass first/median <=1000 ms with complete comparison to the original scalar buffer; see archive/2026-11-05-parallel-effect-passes/verification.md for timing and memory limits.

Review corrections require existing independent profiling references and prohibit generating or overwriting them through the current renderer. Forced same-thread/stolen-half cache regressions and shared raster/vector feathered plain-layer composites cover the nonblocking path. Post-review renderer units (1503) and 32 renderer oracle checks pass; see the verification record for test isolation and both post-review timing attempts.
