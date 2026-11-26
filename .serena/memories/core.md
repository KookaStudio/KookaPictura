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

The #1179 latency follow-up incorporates PR #1221's exact Gaussian/bevel kernels, resolves matte mask gates once, quantizes through bulk plane slices, emits constant-alpha rows in parallel, and replays common effect modes through row kernels. The u8 Gaussian vertical pass uses 16×128 cache-local tiles with unchanged per-pixel tap order. Scalar tests cover tile edges, mask refinements, and all replay components/coverage. Use the ignored production profiling example `effect_cold_latency` for latency acceptance: unit binaries increment a global mask-sample atomic per pixel and can substantially overstate production render time. Three fresh production processes pass first/median <=1000 ms with complete comparison to the original scalar buffer; see archive/2026-11-05-parallel-effect-passes/verification.md for timing and memory limits.

Review corrections require existing independent profiling references and prohibit generating or overwriting them through the current renderer. Forced same-thread/stolen-half cache regressions and shared raster/vector feathered plain-layer composites cover the nonblocking path. Post-review renderer units (1503) and 32 renderer oracle checks pass; see the verification record for test isolation and both post-review timing attempts.

## Tiled planes (#1213)

`Plane<T>` is `Flat(Arc<[T]>)` until `tile(w, h)` / `Plane::filled` / `Plane::build_tiled`; then each channel is 256² tiles (`Empty` | `Uniform` | copy-on-write `Dense`, `tiled_plane.rs`) with per-tile generations. Hot paths use `for_each_tile`, `tiles_mut`(+`_at`), `row()` (a `Cow`: borrowed when flat, copied when tiled), `read_rect_into` / `write_rect_from`, then `settle()` to collapse uniform tiles. `as_slice`/`Deref` on a tiled plane materializes and is counted (`materializations()`); allowed only on `ponytail:` paths. Trace an unexpected count by temporarily printing a backtrace in `plane.rs::materialize`.

Dense tiles are `Arc<Body<T>>` (`tiled_plane/body.rs`, #1217): samples resident, LZ4-packed in RAM, spilled to a checksummed scratch extent (`pictura-core::scratch`), or unreadable. Every access pins the body (`TileRef`'s `TileData::Dense` holds a `DenseRef` guard, so it is not `Copy`; match on `&tile.data`); `tiles_mut` pins in the plane until its next `&mut` call, so callers may move `TileMut::data` out. Never borrow tile samples past a pin (`row_span` copies). `memory::eviction` runs the clock (display drop → history-held → live; pack then spill) above the ceiling; `hot()` holds keep touched tiles resident; `Governor::reserve` reclaims before refusing. Tests that evict take `scratch::tests::serial()` and a `temp_volume` (faults are process-wide).


## Shared coverage seam (#1346)

Implementation is archived at `openspec/changes/archive/2026-11-17-coverage-seam`; the synced capability is `document/coverage-view`. The earlier CMake development build passed on 2026-11-17 (1,425 steps, 80% CPU budget: 12 of 16 CPUs), compiling the app and Qt Test executables with four Rust warnings; those warnings were subsequently addressed in source. Formatting, the active test-plan check, and strict OpenSpec validation passed locally. The user requested commits/PR and all future builds in CI: Rust test compilation, test execution, and remaining gates/evidence are pending in CI. Do not claim verified runtime completion or rebuild locally.

`pictura_core::Coverage` (`coverage.rs`) is the shared read-only editing constraint: signed document bounds, shared `Plane<u8>` storage (or constant coverage), and an exterior byte. `None` at an optional constraint means unrestricted; zero coverage protects pixels. Use `sample`, `read_row`/`read_rect`, `uniform`, `covered_bounds`, and checked `rebased`; a rebase shares storage instead of copying a selection crop. `plane_within` is for actual output planes and preserves constant tiles.

`Selection::coverage` and `Channel::coverage` provide views; `LayerMask::raw_coverage` projects stored values without applying mask metadata. `LayerMask::edit_coverage` resolves the destructive-edit disabled/missing-data policy and returns None for invalid stored geometry. Real mask rendering/refinement and native-depth interpretation keep their actual metadata and existing policy. Render/paint/clipboard/healing selection inputs use Coverage; retained stroke clips use the same core type. The synthetic `selection_to_mask` / `selection_mask_for` helpers are removed. Construct real masks only at actual attachment points; Quick Mask's canonical coverage remains independent of its temporary selection clip.

## Document source and composite values (#1345)

`Document.source: SourceState` owns recorded mode/depth, retained document planes, Indexed palette, converted-away ICC and working ICC. Its internals are private. Use borrowed `planes()`, `original_profile()`, `working_profile()` and `palette_ref()` views; `set_encoding(mode, palette)` and `set_native(depth, planes)` publish correlated changes after validation. A recorded depth alone does not imply retained samples. `Document::document_color_mode`, `document_bit_depth`, `retains_source_depth` and `authoritative_plane` are the shared decisions; retention still scans descendant layer stores, and authority still requires matching recorded depth with no converted mode.

`Document.composite: CompositeState` provides read-only access to its PixelBuffer (also via `buffer()`/Deref). `replace`/`replace_data` and scoped candidate edits validate internal geometry; there is no DerefMut. Its merged-output policy is independent of buffer presence. `detach_plane` and checked `attach_plane` preserve dimensions, channels, output policy and the fixed history plane slot without copying samples. `Document::has_stored_composite`, `composite_shape_is_valid` and `composite_is_authoritative` answer distinct questions; render forwards these decisions. Codec normalization is in `read/normalize.rs`. App saving prepares absent composites through the existing save_view path; direct codec writers retain their buffer preconditions.
