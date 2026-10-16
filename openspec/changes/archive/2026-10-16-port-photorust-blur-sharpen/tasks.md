# Tasks: port-photorust-blur-sharpen

## 1. Evaluate

- [x] 1.1 Route each filter through photorust; run the ImageMagick differentials and GPU parity; record the decisions in `design.md`.

## 2. Engine

- [x] 2.1 Surface Blur and Sharpen Edges on the photorust engine; Kooka's versions removed.
- [x] 2.2 Blur, Blur More, Sharpen, Sharpen More as fixed Gaussians and Unsharp Masks; the 3×3 convolution helper removed.
- [x] 2.3 Median on a clamped sliding histogram.
- [x] 2.4 Shared Gaussian rows on rayon.

## 3. GPU

- [x] 3.1 Blur and Sharpen plans reuse the Gaussian and Unsharp plans; the `KERNEL` repeat loop goes.
- [x] 3.2 Surface Blur shader rewritten to the thresholded mean; GPU radius cap 16.

## 4. Verification

- [x] 4.1 ImageMagick differentials for Blur, Blur More, Sharpen, Sharpen More; mapping table and `tests/README.md` updated.
- [x] 4.2 Unit tests: the fixed filters equal their Gaussian / Unsharp Mask; the histogram median equals the clamped sort; repeated Sharpen stays bounded.
- [x] 4.3 `tests/ported` covers Surface Blur and Sharpen Edges.
- [x] 4.4 `gpu_filter_parity` (with `PICTURA_GPU_BENCH=1`) green; wide Surface Blur falls back to the CPU.
- [x] 4.5 `filter_profile_1024` before and after recorded in `proposal.md`.
- [x] 4.6 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
