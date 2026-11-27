# Design

## Context

See `proposal.md` for the measured problem. The current `qt-headless` job restores only the Cargo registry through `Swatinem/rust-cache` with `cache-targets: "false"`. Corrosion builds under `build/cargo`, so that action cannot restore the app's compiled Rust output. CMake also regenerates moc and autogen files and fetches CxxQt and Corrosion into the build tree.

The CI configure defaults to `RelWithDebInfo` with `BUILD_TESTING` on. That selects the `dev-app` Cargo profile, while `CARGO_INCREMENTAL=0` disables incremental Rust compilation. CI passes `CMAKE_DISABLE_PRECOMPILE_HEADERS=ON`. The app manifest already enables `pictura-render`'s `gpu` feature.

## Goals / Non-Goals

**Goals:**

- Reuse the actual tree that Ninja, Cargo, cxx-qt, moc, and FetchContent produce.
- Keep normal source edits on one restore prefix.
- Reject incomplete or incompatible restores before they can influence a build.
- Measure quota, restore cost, rebuild scope, and Build duration without inventing a runtime target.

**Non-Goals:**

- A custom cache service, daemon, or build system.
- A ten-minute `qt-headless` claim.
- Changes to PCH, GPU, compiler, linker, Cargo profile, or Qt Test registration.
- Immediate absorption of #1056. Its Rust target-cache question stays open until this broader path has measurements.

## Decisions

### 1. Cache one `build/` tree and put fetched sources inside it

Use `actions/cache/restore@v6` and `actions/cache/save@v6` with `path: build`. Configure with `-DFETCHCONTENT_BASE_DIR="$GITHUB_WORKSPACE/build/fetchcontent"` so CxxQt, Corrosion, and their fetched sources live under the cached path. The tree includes:

- `build/cargo` for Corrosion's Cargo target, build scripts, and generated cxx-qt output.
- `build/fetchcontent/_deps` for fetched sources.
- CMake's per-target autogen and moc directories.
- `build.ninja`, `rules.ninja`, `.ninja_log`, `.ninja_deps`, and `CMakeCache.txt`.
- `pictura`, the shared shell, and every Qt Test executable.

Remove `build/qt-test-results` and `build/Testing` only after the test and report steps succeed, immediately before saving. Cached test reports would be stale and are not build inputs.

A target-only cache was rejected because Corrosion's target is outside the workspace `target/` directory. Separate dependency and object caches were rejected for the first measurement because a partial restore complicates correctness and quota attribution.

### 2. Use a stable identity prefix and a unique save suffix

The restore prefix is:

```text
qt-headless-build-v1-<os>-<arch>-<identity-digest>-
```

The full key appends `github.run_id` and `github.run_attempt`. Pull requests restore by prefix. A successful `master` run submits a save under its unique full key, and the measurement records whether quota accepted it. This avoids immutable-cache overwrite conflicts on reruns and lets GitHub evict old entries under its normal quota policy.

The identity digest combines:

- `runner.os`, architecture, and the pinned runner image label.
- `rustc -vV`, `c++ --version`, `cmake --version`, `ninja --version`, `mold --version`, and `ld.lld --version`.
- `qmake6 -query QT_VERSION`, `QT_INSTALL_PREFIX`, and `QT_INSTALL_HEADERS`.
- `Cargo.lock`, root and crate `Cargo.toml` files, `.cargo/config.toml`, and `rust-toolchain.toml`.
- `CMakeLists.txt`, `cmake/**/*.cmake`, the Qt Test `CMakeLists.txt`, `crates/pictura-app/build.rs`, `crates/pictura-app/bridge_sources.txt`, the workflow, and the cache helper.
- Explicit profile and configure values: `RelWithDebInfo`, `BUILD_TESTING=ON`, `CMAKE_DISABLE_PRECOMPILE_HEADERS=ON`, `CMAKE_CXX_COMPILER_LAUNCHER=sccache`, `CARGO_INCREMENTAL=0`, `RUSTC_WRAPPER=sccache`, the `dev-app` profile, and GPU enabled.

Ordinary Rust, C++, header, bridge, and test source content is not hashed into the prefix. Build-graph inputs are hashed because adding or removing a source can change Ninja's graph. Source content is handled by Ninja after restore.

Alternatives considered:

- Hash every source file. Rejected because every pull request would miss the cache.
- Save one immutable key forever. Rejected because the baseline would age and rebuild cost would grow.
- Restore before checkout. Rejected because checkout's clean step can delete the tree and because changed files need reliable post-checkout timestamps.

### 3. Store a content manifest and normalize source timestamps

After all tests pass, `scripts/ci_build_cache.py write-manifest` records:

- Schema, cache key, fingerprint, source commit, and save time.
- SHA-256 for every tracked regular source file and a signature for every tracked source directory.
- Required CMake and Ninja files, the verified configure policy, Corrosion output, FetchContent roots, the app, the shell, and every executable named by CTest's JSON registration.
- Generated roots discovered from `compile_commands.json` include flags and autogen directories, plus SHA-256 for every file under those roots.
- The resolved `CMAKE_HOME_DIRECTORY` from `CMakeCache.txt`.

After restore, `restore` verifies the fingerprint, source directory, required files, executable bits, generated roots, and generated digests. Any failure returns a stale result. CI removes `build/` and continues with a cold configure.

A valid restore then hashes the current tracked tree. Byte-identical files receive an epoch one second older than the oldest required or generated output. Changed and new files receive the current time. Directory signatures get the same treatment so `CONFIGURE_DEPENDS` can still see added or removed files. Symlink targets are not touched.

This is needed because Git does not store checkout timestamps. Without normalization, every fresh checkout can look newer than every restored object. Touching all outputs instead was rejected because it would hide real source edits.

### 4. Keep one writer and save only after success

`qt-headless` remains a single job without a build matrix. The workflow concurrency group prevents overlapping runs for the same ref. Configure, build, self-tests, CTest, reporting, control e2e, manifest creation, and save run in order. Only `master` reaches the save action, and only successful jobs reach it.

The save action uses `continue-on-error` so a quota refusal does not turn a correct build into a failed check. The measurement record still reports the save outcome. No local or cross-worktree writer shares the CI workspace.

### 5. Measure the things that decide whether this pays

The workflow records:

- `cache_state`: `hit`, `miss`, `forced-cold`, or `stale-rejected`.
- `change_classes`: build-input, bridge-file, other Rust, shell-header, Qt Test, or other. Bridge-file means a changed path listed in `bridge_sources.txt`, not proof that the bridge module text changed.
- Restored and saved uncompressed bytes from `du -sb build`.
- Saved archive bytes from the GitHub cache API when the entry is visible. The cache action log remains the fallback.
- Restore, validation, and upload wall time from timestamps around the cache action, manifest validation, and save action.
- sccache hits, misses, and hit rate after Build.
- Ninja edge count parsed from the final build progress line.
- Build duration in seconds.

The job writes `qt-build-cache-metrics.json`, uploads it as an artifact, and appends a summary table. CTest registration is captured with `--show-only=json-v1`; the reporter receives that JSON so a missing per-entry JUnit file fails.

A `workflow_dispatch` boolean named `force_cold` bypasses restore for the required cold measurement. It does not change the configure flags.

## Risks / Trade-offs

- Source hashing and the generated-file digest manifest add setup work and cache metadata. Hashing is bounded to tracked regular files plus generated roots. Measure validation time as part of restore, and drop generated digests only if cache integrity checks and measurements prove they add more cost than protection.
- A compressed build entry may exceed the repository's shared Actions quota or take too long to restore. Record archive bytes and restore seconds on the first master runs. If it does not pay, split immutable FetchContent sources from rolling build outputs or prune nonessential outputs before widening the change.
- Prefix restore can select an older master entry if cache listing order changes. The manifest still proves compatibility, and the matched key is recorded. If baseline age becomes a problem, add bounded retention after the quota measurement.
- Absolute paths in CMake and Corrosion output can make a tree nonportable. The manifest rejects a different `CMAKE_HOME_DIRECTORY`; the current GitHub workspace path is stable.
- A file listed in `bridge_sources.txt` can change only implementation code. The measurement labels that as `bridge-file` and does not claim a bridge-module edit.
- This change overlaps the cache region in `.github/workflows/ci.yml` with #1056 and other build-performance work. Rebase before merge and re-run the warm and cold measurements after conflict resolution.
- Cache save happens after the full test layer. A failed test does not replace the last good tree, but it also means cache upload time is outside the Build measurement and inside job duration.

## Migration Plan

1. Merge the workflow and helper together so no commit enables restore without validation.
2. The first master run is cold, creates a manifest, submits a cache save, and records quota and timing data.
3. Run a pull request against that entry with no source change, then with Rust, shell-header, bridge-file, and Qt Test edits.
4. Run a manual forced-cold job and compare Build duration, Ninja edges, sccache hit rate, restore time, and archive size.
5. Keep, split, or prune the cache from those numbers. Update #1056 only after the broader cache path is proven.

Rollback is a workflow revert. Cached entries can remain untouched because no restore step will reference them.
