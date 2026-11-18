# Tasks

## 1. Tests

- [x] 1.1 Add `tst_control_editing::exportWebpModes` for the lossy default,
  the VP8L `lossless` flag, and the unchanged document state.
- [x] 1.2 Extend `tst_control_editing::exportIsGatedLikeASave` for
  `export_webp`.
- [x] 1.3 Extend `scripts/verify-control.sh` to export WebP through the live
  control socket and assert the `VP8 ` and `VP8L` chunks.

## 2. Control surface

- [x] 2.1 Add the `export_webp` Rust bridge and control method with path
  validation and atomic output.
- [x] 2.2 Register the method as a Save-tier request and a file-writing method
  in the gate.

## 3. Verify

- [x] 3.1 Run `tst_control_editing`, `scripts/verify-control.sh`, clippy, fmt,
  and the OpenSpec validation; archive the change.
