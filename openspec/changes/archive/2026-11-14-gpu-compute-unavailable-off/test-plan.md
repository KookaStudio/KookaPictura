# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `gb_no_gpu_unchecked` | Build a window on a machine with no GPU candidate, create a document, open the Performance preferences page, then call `saveSessionNow()` | Use GPU Compute is unchecked and disabled; the active view reports `gpu_compute()` false; `loadSession().gpuCompute` is false | `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` |
| `gb_safe_mode_stored` | On Windows with a cached `crashed` backend (safe-mode reset path) and a stored `gpuCompute` of true, launch, open the Performance page, then save | The toggle is enabled and checked; the status label is `CPU (safe mode)`; the saved value stays true | manual: no Linux or test path sets the backend-chain reset flag; requires a Windows launch whose chain ended in safe mode |
| Disabling forces the CPU backend | Composite a document with `gpuCompute` false | Every call returns `Backend::Cpu` with a buffer within ±1 LSB of `composite_rgba(doc)` | `pictura-render::disabled_gpu_returns_cpu_backend` |
| Toggle command reflects the value | Ask the command registry for the old `view.gpuCompute` command | The command is not offered; the Preferences > Performance checkbox is the toggle | `tst_command_tree::menuMoves` |
| Toggle reflects the value | Open Preferences > Performance, toggle Use GPU Compute off | The preference flips to false; the active view and checkbox reflect it; `loadSession().gpuCompute` is false | `tst_command_tree::preferencesPages` |
| Document opens without a GPU | Open a document on a machine where no candidate is usable | It composites on the CPU without a panic or error dialog; on a no-candidate host the status bar shows `CPU (no GPU)` | `tst_gpu_backend::statusBarShowsTheBackendLabel` |
| Toggle stays disabled without an adapter | Build a frame with no GPU candidate, open Preferences > Performance, and inspect Use GPU Compute | Use GPU Compute is disabled and unchecked; the active view does not request GPU compute | `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists` |
| `gb_safe_mode_toggle` | On Windows with a cached `crashed` backend, launch, open Preferences > Performance, toggle Use GPU Compute off and on, then relaunch | The toggle is available, the cache is cleared, and the next launch retries the candidates | manual: requires a Windows launch whose chain ended in safe mode to verify the toggle is available |

Automated no-candidate coverage runs on the CPU-only CI hosts; a GPU developer host skips the disabled branch in `tst_gpu_backend::toggleIsDisabledOnlyWhenNoBackendExists`.
