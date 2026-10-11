# Tasks: large-image-filter-preview

## 1. Measurement

- [x] 1.1 `large_document_filter_profile` (ignored) at 16507×16196, CPU and GPU.

## 2. Kernels and the layer wrapper

- [x] 2.1 Row-parallel crop and write-back; straight copy without a mask or transparency lock; skip the alpha pass over an opaque crop.
- [x] 2.2 Row-wise Gaussian; row-parallel Median, Motion Blur, Box Blur, Maximum/Minimum, Custom, High Pass, Unsharp Mask combine.
- [x] 2.3 Byte-identity digests from the serial kernels (`kernel_digests`); opaque-to-opaque test for every moving kernel.
- [x] 2.4 Layer filters skip the GPU for the CPU-faster kernels.

## 3. Previews

- [x] 3.1 Section baselines: save/restore the section only; refresh the union of the written and restored sections.
- [x] 3.2 `FilterProxy::render` by layer path; cached canvas proxy bridge; `ImageView` filter-preview overlay.
- [x] 3.3 Dialog proxy mode below 100% zoom over more than 4 MP; scaled pixel parameters; exact pane section.
- [x] 3.4 No full-resolution reads in the filter, Lens Flare and Lighting Effects dialogs.
- [x] 3.5 Coalesce parameter changes.

## 4. Feedback round

- [x] 4.1 History records "Filter: <name>" and "Filter Gallery: <names>".
- [x] 4.2 Commits on a worker (filter, composite, history) with a busy window; the UI thread only swaps results in.
- [x] 4.3 Exact zoomed-out preview from a worker replaces the draft; latest request wins; commit reuses it; pane section rendered off the document.

## 5. Second feedback round

- [x] 5.1 History deltas keep one side per tile; capture diffs into `current`; moving walks swap, rebuilding walks read.
- [x] 5.2 Model test rebuilds every state after every step; the retained-bytes bound halves.
- [x] 5.3 Radial Blur: tiled, shared taps, interleaved planes, precomputed Spin rotations; digests recorded from the serial kernel.
- [x] 5.4 Radial Blur Zoom on the GPU (±1 LSB); Spin stays on the CPU.

## 6. Verification

- [x] 6.1 Rust tests: section restore, in-place planes, canvas proxy, CPU routing, background commit, exact preview reuse and supersession, off-document section.
- [x] 6.2 Qt tests: zoomed-out preview through the menu (draft, exact, cancel, commit, label), scaled parameters, gallery label, adapted synchronous-preview checks.
- [x] 6.3 `scripts/verify-fast.sh` and `openspec validate --all --strict`.
