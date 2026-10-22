# Tasks: System Info panel

## 1. Tests first

- [x] 1.1 Add the Rust `gpu_adapter_info` unit named in `test-plan.md`.
- [x] 1.2 Add the `tst_system_info` cases named in `test-plan.md`.
- [x] 1.3 Register `tst_system_info` in the Qt Test CMake list.

## 2. Engine and bridge

- [x] 2.1 Expose `pictura_render::gpu_adapter_info()` and `GpuAdapterInfo` from the cached adapter.
- [x] 2.2 Add the `gpu_adapter_info()` cxx-qt invokable returning the five adapter fields.

## 3. App

- [x] 3.1 Add `system_info.{h,cpp}`: the value, Linux parsers, collector, report formatter, and issue URL.
- [x] 3.2 Add `system_info_dialog.{h,cpp}`: read-only report, Copy, Close, and refresh.
- [x] 3.3 Add `frame_system_info.cpp` and wire the two Help commands.
- [x] 3.4 Add the new C++ files and the build/version definitions to CMake.

## 4. Docs and metadata

- [x] 4.1 Update the Help-menu and diagnostics docs to record the shipped surface.
- [x] 4.2 Update `ROADMAP.md` and `docs/dev/STATE.md`.

## 5. Gates

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (3602 passed, 23 skipped; the one failure is the pre-existing local ImageMagick fixture mismatch).
- [x] 5.2 Build the app and run `tst_system_info`.
- [x] 5.3 `openspec validate --all --strict`, then archive the change.
