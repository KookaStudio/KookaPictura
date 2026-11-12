# Test plan

This plan covers the ADDED `document/out-of-core-transform` requirement of
`move-commit-spill-gate` (#1290).

**Fixtures.** The Rust gate test pins the governor's host reading and the
scratch volumes' free bytes, as the transform gate tests do. The Qt tests pin
the same through the bridge (`set_available_memory_for_test`,
`set_scratch_available_for_test`) and commit through `PictureView::commit_move`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `oct_move_gate_spills` | Gate `move_commit_cost` of a 17000² full canvas moved by (5, 5) with 1.75 GiB room and 64 GiB scratch | Admitted; `12 · area` exceeds the room | `pictura-app::a_large_layer_move_spills_its_composite` |
| `oct_move_commits` | New 4100² document, duplicate the background, pin room between `4 · area` and `12 · area` and no scratch, `commit_move(5, 5)` | History grows by one; no memory refusal | `tst_memory_governor::moveChargesOnlyTheTiledComposite` |
| `oct_move_scratch_full` | Same document; pin room above the working set but below `4 · area`, no scratch; `commit_move(5, 5)` | Refused; scratch-full recorded; history unchanged; `moveMemoryRefusalMessage()` names the scratch disks | `tst_memory_governor::moveRefusalNamesFullScratchDisks` |
| Repeated moves on a 16000² document | Open a 16000 × 16000 document with a 13000² gradient layer; drag it with the Move tool ten times | No "not enough memory" refusal; every move lands in History | manual: needs a 256 MP document and several GB of RAM and scratch, too large for CI |
