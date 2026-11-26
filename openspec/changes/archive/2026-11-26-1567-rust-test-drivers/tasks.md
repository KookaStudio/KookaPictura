# Tasks

## 1. Planning and review

- [x] 1.1 Author the proposal, design, capability delta, tasks, and scenario test plan; verify every scenario has a tagged test-plan row.
- [x] 1.2 Run the proposal review and fix every legitimate finding; verify the reviewed artifacts match the files on disk.
- [x] 1.3 Rebase onto current `origin/master` and fold the two render suites #1612 added (`codec_content_roundtrip`, `codec_layer_oracles`) into the `document` and `oracle` drivers; verify no test from the newer base is deleted.

## 2. Codec topic drivers

- [x] 2.1 Move the camera RAW suites and their `dng/` and `raw/` helper directories under `tests/camera_raw/`; verify every former RAW suite is declared by `main.rs` and retains its relative helper paths.
- [x] 2.2 Move the metadata, PSD, and save suites into their topic directories; verify every moved suite is declared exactly once and PSD fixture includes resolve from the new depth.
- [x] 2.3 Keep `oracle.rs`, `hostile_inputs.rs`, `streamed_save.rs`, and `merged_preview_buffers.rs` as top-level binaries; verify the three allocator suites still own their `#[global_allocator]` declarations.

## 3. Render topic drivers

- [x] 3.1 Move the document and PDF suites, including `cold_compare_exit`, into `tests/document/` and `tests/pdf/`; verify each driver declares every moved suite exactly once.
- [x] 3.2 Move `oracle.rs` to `tests/oracle/main.rs` and add the five former oracle binaries as modules; verify the binary name stays `oracle` and its fixture paths still resolve.
- [x] 3.3 Keep the backend, canvas-mask host, cold-process, GPU, and text-gamma suites as top-level binaries; verify none shares a driver with conflicting process-wide state.

## 4. Profiling examples

- [x] 4.1 Move the eight named profiling suites to `examples/`; verify each has a `main` entry point and the two GPU-gated files retain their feature gate.
- [x] 4.2 Convert test attributes to conditional test attributes and add example entry points; verify every original test function and exact ignore reason remains in source, including all three `source_cache_profile` entry points.

## 5. Commands and references

- [x] 5.1 Update current READMEs, active specs, agent guidance, and the Serena command memory to the new paths and driver names; verify quoted commands name existing targets.
- [x] 5.2 Update archived OpenSpec commands and moved profiling paths; verify no archived command still selects one of the eight profiling files with `--test`.
- [x] 5.3 Limit `docs/` edits to required run-string or moved-file references; verify the final report flags them for a later `TASK-ALLOWS-DOCS` commit.

## 6. Source review and handoff

- [x] 6.1 Compare the pre-change and post-change test-name and ignore-reason inventories from source; verify no non-ignored test or reason string was lost.
- [x] 6.2 Inspect target discovery, retained nextest GPU filters, file sizes, dependencies, and allowlist changes from source; verify codec and render integration binary counts fall without manifest target entries.
- [x] 6.3 Run the change review, fix every legitimate finding, and inspect the final diff; verify the worktree is review-clean, unverified, uncommitted, and unarchived.
- [x] 6.4 Add `required-features = ["gpu"]` to the two GPU-gated examples and `#![allow(clippy::duplicate_mod)]` to the camera RAW driver; verify a default `cargo build --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [x] 6.5 Prove inventory preservation against `origin/master` with `cargo nextest list`: 3384 non-ignored tests before and after, and the eight profiling test names present as example test targets.
- [x] 6.6 Run the device-free examples through `cargo run --example` and the GPU-gated ones via `--features gpu`; verify each prints its profiling output.
- [x] 6.7 Run `LC_ALL=C bash scripts/verify-fast.sh` under the wrapper and `openspec validate --all --strict`; verify any remaining failure also fails on a clean `origin/master` worktree.
- [x] 6.8 Archive this change once the implementation is committed, and record the archive path in the PR.
