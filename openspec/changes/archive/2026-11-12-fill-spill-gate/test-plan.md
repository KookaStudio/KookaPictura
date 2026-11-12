# Test plan

This plan covers the ADDED `document/out-of-core-fill` requirements of
`fill-spill-gate` (#1300).

**Fixtures.**

- **Rust gate tests:** pin the governor's host reading and the scratch volumes'
  free bytes, as the transform and Move gate tests do, and gate
  `memory_gate::fill_cost` directly.
- **Qt memory tests:** pin the same through the bridge
  (`set_available_memory_for_test`, `set_scratch_available_for_test`) and call
  the fill bridge functions on a 4100² document. That is over 16 Mpx, so a
  whole-canvas refresh only damages tiles.
- **Qt job tests:** drive the Gradient tool and Edit > Fill through the shell,
  with `document_job_set_delay` holding the job, and watch the `jobProgress`
  window as `tst_document_jobs` does for whole-document commands.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ocf_gate_spills` | Gate `fill_cost` of a Gradient over a full-canvas layer of a 17000² RGB/8 document, with 1.75 GiB of room and 64 GiB of scratch | Admitted; `fork_bytes + refresh_bytes` exceeds the room | `pictura-app::a_large_fill_spills_its_tiles` |
| `ocf_small_refresh` | Compute `fill_cost` of a Gradient over a full-canvas layer of a 2000² RGB/8 document | `working` is `refresh_bytes` of the canvas | `pictura-app::a_small_document_fill_charges_its_refresh` |
| `ocf_fills_commit` | 4100² document; for each of `draw_gradient`, `bucket_fill_at`, `edit_fill`, and `edit_stroke` (with a selection inset 8 px), pin room above the fill's working set but below its old RAM charge, with 4 GiB of scratch | Each adds one history state; no memory refusal | `tst_memory_governor::fillsSpillToScratch` |
| `ocf_scratch_full` | Same document; pin 16 MiB of room and no scratch; `draw_gradient` across the canvas | Refused; scratch-full recorded; history unchanged; `fillMemoryRefusalMessage()` names the scratch disks | `tst_memory_governor::fillRefusalNamesFullScratchDisks` |
| `ocf_fill_job` | 2400 × 1800 document, job delay 800 ms; drag the Gradient tool across the canvas | `jobProgress` shows a busy bar; one `"Gradient"` state is added; pixels changed | `tst_document_jobs::aLargeGradientRunsAsAJob` |
| `ocf_fill_cancel` | 2400 × 1800 document, job delay 30 s; dispatch Edit > Fill with its dialog accepted; press Cancel when the window shows | History count and index are unchanged; displayed pixels are identical | `tst_document_jobs::cancellingAFillChangesNothing` |
| `ocf_fill_small_sync` | 64 × 48 document; drag the Gradient tool | No job pending; no window; `"Gradient"` recorded on return | `tst_document_jobs::aSmallGradientFillsAtOnce` |
| Gradient on a 17000² document | Open a new 17000 × 17000 RGB/8 document on a host with less free RAM than its tiles, with a scratch disk set; drag the Gradient tool across a selection | A busy progress window, then the gradient lands in History; no "not enough memory" message | manual: needs a 289 MP document and several GB of RAM and scratch, too large for CI |
