# Test plan

This plan covers `document/out-of-core-image-size`, and the modified
`document/image-resize-memory-budget` and `verification/memory-benchmarks`
requirements, for `image-size-out-of-core` (#1293).

**Fixtures.** The engine tests use seeded documents and planes. The dense
resamplers stay as `#[cfg(test)]` oracles: `document_ops::resample_plane`, which
calls `pictura_ops::resize`, for 8-bit planes, and
`native_store::resize_samples` for retained stores. Under `cfg(test)` the
window budget is 4096 samples, so every test resize also splits its blocks. The
gate tests pin the host's available memory and the scratch volumes' available
bytes. The spill test configures a temporary scratch volume and runs
`eviction::run_pass`. The Qt memory-refusal tests pin 260 MiB of available memory (4 MiB of room, under
one 30000-wide output strip) and no scratch space; the scratch-full test pins
1.75 GiB and no scratch, so its working set fits and its planes do not. Neither
depends on the host.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ocr_resize_exact` | Resize a seeded 8-bit document (a full-canvas layer, an offset layer with a mask, an alpha channel) to 150 %, 50 %, 7 % × 31 % and 233 % × 100 %, with Nearest, Bilinear and Bicubic | Every channel, mask and alpha channel tiled; equal to the dense resampler byte for byte | `pictura-render::tiled_image_size_matches_the_dense_resampler` |
| `ocr_resize_deep` | Resize a 16-bit and a 32-bit document, each with a retained layer store holding a `-2` plane and a retained document store, to 150 % Bicubic | Every retained plane tiled and byte-identical to `resize_samples` | `pictura-render::retained_stores_resize_through_tiles` |
| `ocr_uniform_tiles` | Resize a 2048² solid document to 150 % | Each channel allocates under a quarter of the dense plane | `pictura-render::a_solid_document_resizes_into_uniform_tiles` |
| `ocr_output_spills` | Resize a 1024² seeded document to 150 % with a scratch volume, then run eviction passes | Some tiles spilled; the layer reads back equal to the dense resampler | `pictura-render::resized_tiles_spill_and_read_back` |
| `ocr_memory_max` | `python3 scripts/profile-tools.py --scenarios M15 --memory-max 2G`, then `--memory-max 4G`, at 17000² | `outcome: ok` in both, with no OOM kill or refusal | manual: needs a 17000² document under a systemd cgroup memory limit and several minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
| `ocr_gate_spills` | Gate the cost of resizing a 17000² full-canvas-layer document to 150 % at 1.75 GiB room and 64 GiB scratch | Admitted; `expects_spill` holds | `pictura-app::a_large_image_size_is_admitted_when_scratch_holds_it` |
| `ocr_gate_scratch_full` | Gate the same resize at 1.75 GiB room and no scratch | Refused; `edit_refused_for_scratch_full` | `pictura-app::full_scratch_disks_refuse_a_large_image_size` |
| `rs_refuses` | New 40×20 RGB/8 document; pin 260 MiB available and no scratch; `image_size_apply` with `bicubic` to 30000×30000 | Returns false; width, height and history count unchanged | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_bridge_refuses` | New 40×20 document; the same pins; `PictureView::resize_image` to 30000×30000 | Returns false; the document and its history unchanged | `tst_image_ops::resizeImageRefusesAnOverBudgetTarget` |
| `rs_allows` | New 40×20 document; accept Image Size at 20×10 | Returns true; 20×10; current label "Image Size" | `tst_image_ops::imageSizeResamplesAndSetsResolution` |
| `rs_resolution_only` | After the refusal, `image_size_apply` with no resample at 40×20 and 300 ppi | Returns true; 40×20 at 300 ppi | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_side_limit` | Pin 64 GiB available and 64 GiB scratch; ask `image_size_fits_budget` about 30001×10 | False | `tst_image_ops::imageSizeRefusesASideOverTheLimit` |
| `rs_message` | With the pins, ask the gate about 30000×30000 and read the warning text | `imageSizeFitsBudget` is false; the text names 30000 and memory | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_warning_shown` | With the pins, dispatch `image.image.size`, answer the dialog with 30000×30000, then answer the warning | One warning is shown; the document and its history are unchanged | `tst_image_ops::imageSizeShowsTheBudgetWarning` |
| `rs_scratch_message` | Pin 1.75 GiB available and no scratch; ask the gate about 30000×30000 | False; the message names 30000 and the scratch disks | `tst_image_ops::imageSizeNamesFullScratchDisks` |
| `rs_shared_budget` | Shared predicate against synthetic budgets; the resize bound against low, high and unknown room | Sides first, byte limits enforced, inside accepted, the resize bound drops the floor | `pictura-app::budget_judges_sides_then_bytes`, `pictura-app::budget_honours_the_document_limit`, `pictura-app::a_resize_document_limit_drops_the_allocation_floor` |
| `mb_catalogue` | `profile-tools.py --self-check` with a built app | Every M1–M15 row `outcome: ok` at 512² | `profile-tools::self_check_catalogue` |
