# Tasks

## 1. The seam types and verbs

- [x] 1.1 Declare `Refusal { Memory, ScratchUnreadable, ScratchFull }` and `Admission { Admitted { spill }, Refused(Refusal) }` in `helpers/memory_gate.rs`, with one GUI-thread `LAST_REFUSAL` cell replacing the two refusal booleans. The governor's refusal remains only a fallback when the seam has no typed reason; `SCRATCH_AVAILABLE` and `UNREADABLE_SEEN` are not refusal state and stay.
- [x] 1.2 Implement `admit_resident(bytes) -> Admission` and `admit_commit(cost) -> Admission`, each clearing the cell first, and record `Refusal::ScratchFull` when spillable bytes exceed room plus scratch.
- [x] 1.3 Derive the predicates from `LAST_REFUSAL`: memory accepts `Memory`/`ScratchFull` and rejects `ScratchUnreadable`; scratch and scratch-full select their own cases; only an absent typed reason falls back to the governor. Keep test-only adapters for retained gate tests.
- [x] 1.4 Add the admission, refusal, scratch-full, spill-marking, no-stale and typed-reason-precedence unit tests beside the seam.

## 2. The Quick Mask defect

- [x] 2.1 Add `a_quick_mask_gate_clears_a_prior_refusal`: refuse an edit, restore the room, route a Quick Mask gate through `admit_resident`, and assert no refusal is recorded.
- [x] 2.2 Route `quick_mask_document` through `admit_resident`, and clear `begin_quick_mask_paint_with_tip`, `begin_move_preview` and `commit_move` before their non-memory early returns.
- [x] 2.3 Extend `quickMaskStrokeAfterRefusalShowsNoMemoryMessage`: prove a successful Quick Mask begin, a no-single-target brush failure, and Quick Mask Move begin/commit failures present no stale memory refusal.

## 3. Migrate the history, layer and paint families

- [x] 3.1 Convert `impl_history.rs`, `impl_layers.rs`, `stack_files.rs`, `impl_core.rs` and `impl_paint.rs` to `admit_resident`. Keep a `clear()` where the command can refuse for a non-memory reason before admission; delete it where admission resets anyway.
- [x] 3.2 Convert `paint_tools/imp.rs` to `admit_resident` and `paint_tools/fills/imp.rs` to `admit_commit`, using `Admitted { spill }` where transform code previously asked `expects_spill`.
- [x] 3.3 Preserve the existing fill, layer and memory-governor refusal tests.

## 4. Geometry, transform and save families

- [x] 4.1 Rewrite `helpers/geometry_admit.rs` so its four gates consume one `Admission` from `admit_commit`, dropping redundant clears while retaining the side-limit clear.
- [x] 4.2 Convert the admission call sites in `impl_transform/session.rs`, `impl_transform/selection/mod.rs`, `impl_transform/mask_move.rs`, `impl_paint/mask.rs` and `image_adjust/new_document/imp.rs`. `impl_transform/dispatch.rs`, `impl_transform/bridge/imp.rs`, `image_adjust/image_ops/imp.rs` and `image_adjust/image_size/imp.rs` are unchanged from `origin/master` and already route through `geometry_admit` or their own precondition clears.
- [x] 4.3 Audit `impl_core/save.rs`, `impl_core/save_jobs/imp.rs` and `helpers/save_gate.rs`: they are unchanged from `origin/master` and already use `save_gate` reservations plus precondition clears. Keep `SaveAdmission` and the native save result contract; scratch-full records through the shared typed refusal.
- [x] 4.4 Route `scratch_rollback.rs` through `refuse_for_scratch`, keep the read-count check inside the seam, and add `a_rolled_back_edit_reports_unreadable_tiles`.

## 5. One presentation path

- [x] 5.1 Add `Refusal` to the cxx bridge and present the typed reason, so a scratch-full refusal is never presented as a memory shortfall.
- [x] 5.2 Collapse the four bespoke ladders and geometry commit presentation onto `refusalMessage`/`reportRefusal`, preserving commit presentation's Memory/ScratchFull scope.
- [x] 5.3 Make `everyCommandFamilyShowsItsOwnWording` exercise Fill, Move, Free Transform, Image Size, Undo, Add Layer and Save through their real admission seams, asserting each action's wording and unchanged history where applicable.

## 6. Test pins and surface cleanup

- [x] 6.1 Move the host, cgroup and scratch test pins into a test-only module while keeping the bridge declarations.
- [x] 6.2 Delete `admit_fresh` and any adapter left unclaimed by a real caller.
- [x] 6.3 Move the inline `memory_gate` tests to `helpers/memory_gate_tests.rs` with the repository's `#[path]` test-module pattern, preserving every test and tag.

## 7. Integration verification

- [x] 7.1 Run `cargo fmt --all`, configure with `cmake -S . -B build -G Ninja`, and build with `cmake --build build --parallel` under the CPU-budget wrapper. The first build found one duplicate brush-angle argument in the new Quick Mask Qt case; the corrected rebuild passed.
- [x] 7.2 Review `ROADMAP.md` and `PARITY.md`. This refactor changes no feature coverage, so neither tracker needs an update.
- [x] 7.3 Run `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`. Formatting and Clippy passed. The report ended with 11,840 passed, 152 skipped and one failure, `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`, reproduced identically on clean `origin/master`.
- [x] 7.4 Record the exact aggregate result and run the guards that the baseline test failure prevents the aggregate from reaching. File size, scoped guard, test-plan mapping before archive, archive, parity, build-tree freshness and `openspec validate --all --strict` passed. `check-milestone-names.py --self-test` fails identically on clean `origin/master`. The focused verb-manifest test and `tst_memory_governor` both pass.
