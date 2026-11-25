# Test plan: memory-gate-admission-seam

The seam's own checks are Rust unit tests in
`crates/pictura-app/src/cxxqt_object/helpers/memory_gate_tests.rs`, loaded from
`memory_gate.rs` through the repository's `#[path]` test-module pattern. The
typed-reason precedence regression records a memory refusal and then an
unreadable-scratch refusal, proving the predicates cannot disagree and that the
governor fallback applies only without a typed reason.

End-to-end rejection and presentation are Qt cases in
`crates/pictura-app/cpp/tests/tst_memory_governor.cpp`. Existing refusal tests
remain intact; the Quick Mask defect gets one Rust check and one Qt case that
also covers the Move siblings.

The five `document/memory-governor` scenarios are already covered by tests that
this change must not alter; they are listed with their existing coverage so the
gate proves the refactor left them intact.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `mga_admits` | Supply a cost within a pinned room and scratch and call the resident admission. | The outcome is admitted, no refusal is recorded, and every derived predicate reads false. | `pictura-app::a_fitting_edit_is_admitted` |
| `mga_refuses` | Pin the room below a cost, call the resident admission, then read the predicates and the document. | The outcome is a memory refusal, `edit_refused_for_memory` is true and the other two are false, and the command leaves document/history state unchanged. | `pictura-app::an_edit_beyond_the_room_is_refused`, `tst_memory_governor::everyCommandFamilyShowsItsOwnWording` |
| `mga_scratch_full` | Pin room for a commit's resident bytes but not its spillable bytes with no scratch free. | The outcome is a scratch-full refusal and it is not presented as a memory shortfall. | `pictura-app::a_scratch_full_commit_is_refused_with_its_own_reason`, `tst_memory_governor::fillRefusalNamesFullScratchDisks` |
| `mga_spill_marked` | Admit a commit whose resident plus spillable bytes exceed the room but fit room plus scratch, then admit one the room holds alone. | The first outcome reports spill, the second does not. | `pictura-app::an_admitted_commit_reports_whether_it_spills` |
| `mga_no_stale` | Refuse an edit, restore the room, run an admitted edit that makes no refusal, then read the predicates. | No refusal is reported for the second edit and all predicates read false. | `pictura-app::an_admitted_edit_records_no_refusal`, `pictura-app::a_scratch_refusal_supersedes_an_earlier_memory_refusal`, `tst_memory_governor::quickMaskStrokeAfterRefusalShowsNoMemoryMessage` |
| `mga_quick_mask_clean` | Refuse an edit, restore the room, complete a Quick Mask paint begin, force a no-single-target brush failure, then force Quick Mask Move begin and commit failures. | No command presents the earlier memory refusal; the brush shows its own non-memory message. | `pictura-app::a_quick_mask_gate_clears_a_prior_refusal`, `tst_memory_governor::quickMaskStrokeAfterRefusalShowsNoMemoryMessage` |
| `mga_one_path` | Refuse a fill, a Move, a Free Transform, an Image Size, an undo, an add layer and a save for a memory shortfall with the room pinned. | Each shows its own action wording through the real admission seam and no command records a history state. | `tst_memory_governor::everyCommandFamilyShowsItsOwnWording` |
| `mga_scratch_full_presented` | Refuse a fill, a Move, a Free Transform and an Image Size with full scratch disks, then a legacy (non-native, e.g. PNG) save for the same reason. | Every message says the scratch disks are full and none says there is not enough memory, including the legacy save. | `tst_memory_governor::fillRefusalNamesFullScratchDisks`, `tst_memory_governor::moveRefusalNamesFullScratchDisks`, `tst_memory_governor::transformRefusalNamesFullScratchDisks`, `tst_image_ops::imageSizeNamesFullScratchDisks`, `tst_memory_governor::scratchFullRefusalThroughTheGenericPathNamesTheScratchDisks` |
| `mga_rollback` | Arm an unreadable scratch tile, run an edit that commits and rolls back, then read the recorded reason and typed presenter. | The reason is the unreadable-tiles case and the presented wording names reading back from the scratch disk. | `pictura-app::a_rolled_back_edit_reports_unreadable_tiles`, `tst_memory_governor::everyCommandFamilyShowsItsOwnWording` |
| `mg_gates_refuse` | Pin the room to zero and run New Document, Add Layer, a brush stroke, Undo, Redo, add snapshot, restore snapshot, Save, then Open, then restore the room. | Each is refused, the document and history are unchanged, no file is written, and each runs with room. | `tst_memory_governor::everyPathRefusesWithoutRoom` |
| `mg_fill_under_cgroup` | Pin a 512 MiB cgroup with 400 MiB in use and run a Paint Bucket click inside a marquee. | The fill is refused, no history state is added, and the layer's pixels are unchanged. | `tst_fill_tools::refusedForMemoryShowsStatus` |
| `mg_status_message` | Pin the room to zero and run Add Layer, Undo and New Document from the application, then a fill. | Each leaves its own message on the status bar and the refused fill keeps the fill message. | `tst_memory_governor::refusalsShowTheStatusMessage` |
| `mg_control_refused` | Pin the room to zero and send `document new` and `layer_op add`. | Each answers the error `refused` and the document count and layer count are unchanged. | `tst_memory_governor::controlAnswersRefused` |
| `mg_estimates_cover_peaks` | Run the memory benchmark's M1, M2, M6, M7, M8 and M11 scenarios at 4250² alone in their process. | Each measured heap peak is at most its path's estimate. | `pictura-app::estimates_cover_the_measured_peaks` |

No manual scenarios remain. Every row is automated.

## Verification

Reset ownership stays explicit. `admit_resident` and `admit_commit` clear the prior refusal on every admission. Commands with a non-memory early return, including Quick Mask Move and native save preconditions, keep their boundary `clear()` before that return. The checks do not require deleting those precondition clears.

- `cargo fmt --all`: passed.
- `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --parallel'`: passed after the new Quick Mask Qt case was corrected to pass the generated `begin_paint` signature's seven integer parameters.
- `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`: formatting and Clippy passed; the report ended with 11,840 passed, 152 skipped and one failure, `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`. A clean `origin/master` worktree reproduces the same pixel mismatch.
- `cargo test -p pictura_app verb_manifest_matches_bridge_sources -- --nocapture`: 1 passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_memory_governor$' --output-on-failure`: 1/1 passed.
- Scoped file-size, guard, archive, parity and build-tree checks passed. `python3 scripts/check-test-plan.py` checked the active change before archive and passed.
- `openspec validate --all --strict`: passed separately after the aggregate stopped at the baseline GPU failure.
- `python3 scripts/check-milestone-names.py --self-test`: fails with `a scoped scan missed a changed violation` on both this branch and clean `origin/master`.
