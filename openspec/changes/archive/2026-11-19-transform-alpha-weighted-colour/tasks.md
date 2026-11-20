# Tasks

## 1. Kernel

- [x] 1.1 Add the alpha-weighted sampler to `kernel.rs`, falling back to the plain kernel when every weighted tap is opaque.

## 2. Resampler

- [x] 2.1 Carry an optional alpha plane through `transform_tiled::Source` and read its window alongside the colour window.
- [x] 2.2 Pass the layer's `-1` plane to its colour channels in `transform_layer_with_map` and in the native store.

## 3. Stale store

- [x] 3.1 Refresh each retained store plane that no longer narrows to its 8-bit plane in `prepare_layer` (Free Transform and Warp).

- [x] 3.2 Auto-Align reads luma premultiplied by alpha.

## 4. Tests

- [x] 4.1 Kernel tests: weighted equals plain on opaque taps; a transparent black tap does not darken.
- [x] 4.2 Layer tests: erased-edge scale, rotated-edge colour, and the 16-bit store.
- [x] 4.3 Erase-after-store tests for a scale and a warp.
- [x] 4.4 Auto-Align work-image luma test.

## 5. Verify

- [x] 5.1 Run fmt, clippy, the render tests, the test-plan check, and `openspec validate --strict`; archive the change.
