# Rust–Qt Interop

- **Spec ID:** `ARCH-004` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` (technology-selection document)
- **Depends on:** `ARCH-003` qt6-ui-design, `ARCH-005` threading-and-concurrency, `ARCH-006` gpu-rendering-pipeline

> All crate and module names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Versions and maintenance status reflect the sources
> fetched for this document and must be re-checked before implementation.

## CS6 behavior

Not user-visible. This document chooses how Rust and Qt6 exchange objects,
signals, and models. The only behavioral requirement it inherits is that the
interface described in `ARCH-003` must work: a widget shell with dockable
panels, a custom GPU canvas, and item views backed by document state.

## UI surface

None directly. Every UI surface depends on this decision for correct threading
and object lifetime. The surfaces that stress the bridge most are:

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers / Channels / History panels | Item views | `F7`, etc. | Models implemented on the Rust side |
| Options bar | Tool bar | n/a | Rebuilt on tool change; Rust supplies options state |
| Canvas | Custom GPU widget | n/a | Large buffers must not round-trip through Qt types |
| Progress / status bar | `QStatusBar` | n/a | Long operations report progress from worker threads |
| Menus / shortcuts | `QAction` | many | Actions often call into Rust commands |

## Parameters & ranges

Interop configuration surface.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Bridge | enum | CXX-Qt | CXX-Qt / qmetaobject-rs / manual FFI | Primary decision below |
| Bulk-buffer path | enum | Rust-owned | Rust-owned / Qt image / shared GPU texture | Avoid `QImage` copies per tile |
| Connection type | enum | Auto | Auto / Direct / Queued / BlockingQueued | Queued for cross-thread |
| Worker threads | int | `max(1, cores-1)` | 1 to cores | Reserved for the UI thread |
| Panic policy | enum | Abort at boundary | Abort / Convert to error | Rust panics must not unwind into C++ |

## Algorithms & pipeline

### Candidates

**CXX-Qt.** KDAB's crates for bidirectional Rust/C++ bindings with Qt, built on
the `cxx` crate. It generates C++/Rust glue for implementing `QObject`
subclasses in Rust, with attributes for signals, slots, and properties, usable
from C++, QML, and JavaScript. The project is maintained by KDAB, supports
Qt 5.15 LTS and all Qt 6, and is tested on Linux, Windows, macOS, and WASM.
It documents threading helpers (`CxxQtThread`, a `Threading` trait), connection
handles (`QMetaObjectConnection`, `ConnectionType`), and QML plugin support. Its
own README states it is in **early development and the API changes frequently**.
It provides Rust bindings for QtCore/QtGui via `cxx-qt-lib`, but **no Rust
bindings for the QWidgets API**; QObject subclasses written in Rust can still be
hosted in a C++ Widgets application. Build integration uses `cxx-qt-build` in a
Cargo build script, or CMake; on Linux a faster linker (mold, lld, or ld.gold)
is documented as required.

**qmetaobject-rs.** A Rust framework for QML applications. It builds
`QMetaObject`s at compile time, exposes Rust structs to QML via
`#[derive(QObject)]` and macros (`qt_base_class!`, `qt_property!`, `qt_signal!`,
`qt_method!`), and supports `QQuickItem` and `QAbstractListModel` subclasses.
Missing API is reached through the `cpp!` macro, which inlines C++ into Rust.
It requires Qt 5.8 or newer and needs no separate build system beyond Cargo plus
`cpp_build`. Critically for this project: it "focuses solely on QML, not
QWidgets", and its README states it is **"currently only being passively
maintained as focus has shifted towards Slint"**, with the author pointing users
to Slint.

**Manual C FFI.** Hand-written `extern "C"` declarations over Qt, or the `cxx`
crate on its own, with signals and slots wired through `QMetaObject` and
`QMetaObject::invokeMethod`. There is no code generation and no framework. It
offers the most control and the smallest dependency surface, at the cost of
hand-writing object lifetime, thread affinity, and signal plumbing for every
type. It is well suited to a narrow, stable, Qt-free boundary (for example, bulk
pixel buffers) but expensive as the application-wide bridge.

### Comparison

| Criterion | CXX-Qt | qmetaobject-rs | Manual FFI |
|---|---|---|---|
| Maintained | Yes (KDAB) | Passive | Depends on us |
| Intended for | Rust in C++/Qt, or Rust apps | QML-only Rust apps | Narrow boundaries |
| QWidgets host | Yes (QObject subclasses used from C++) | No Rust QWidgets | Yes, hand-written |
| QML types | Yes | Yes | Hand-written |
| Safe Rust | Yes | Yes | Only if wrapped carefully |
| Signals/slots | Attributes + generated glue | Macros | Manual `QMetaObject` |
| Properties | Yes | Yes | Manual |
| Threading helpers | `CxxQtThread`, `Threading` | Manual (queued invokes) | Manual |
| Build | Cargo script or CMake; needs fast linker | Cargo only (`cpp_build`) | Cargo + build script |
| Stability | Early, API churn | Stable-ish, slowing | Under our control |
| Escape hatch | `cxx` bridge | `cpp!` | n/a |

### Recommendation

Use **CXX-Qt as the application-wide bridge**, backed by a **narrow, Qt-free
`cxx`/FFI boundary for bulk pixel and tile buffers**.

Reasons:

1. The application is a **Widgets** app (`ARCH-003`). qmetaobject-rs explicitly
   does not target QWidgets, so adopting it would leave the entire panel and
   frame layer without a Rust bridge.
2. CXX-Qt is the only candidate that is both actively maintained and designed for
   Rust/C++ bidirectional Qt bindings with signals, slots, and properties. Its
   `QObject` subclasses are usable from C++, QML, and JavaScript, which matches
   a shell that is partly C++ and partly Rust.
3. Its threading helpers (`CxxQtThread`, `Threading`) map directly to the
   requirements in `ARCH-005` for queuing worker results onto the GUI thread.
4. Its cost, an early and changing API plus a build that needs a fast linker, is
   acceptable for a long-lived internal project and can be pinned to a version.

The manual boundary is still used for hot data. Per-tile image buffers cross as
`(pointer, length)` over a `cxx` bridge or as shared GPU textures
(`ARCH-006`); they never become `QImage`/`QVariant` per tile. Qt value types
(`QString`, `QByteArray`, `QVariant`, and small structs) cross through
`cxx-qt-lib` where they are not hot.

Fallbacks:

- If CXX-Qt API churn becomes a maintenance problem, the bridge is confined to
  a `pictura_qt` crate so it can be replaced with manual FFI without touching
  the core.
- If the project ever drops Widgets for a QML-only shell, qmetaobject-rs becomes
  a viable, simpler alternative, but that is a larger product decision.

### Thread affinity

Qt requires that a `QObject` be used from the thread it lives in. The bridge
must enforce:

- All `QObject`, model, and widget access occurs on the GUI thread.
- Worker threads send results through queued connections (CXX-Qt
  `ConnectionType::Queued` / `CxxQtThread`) or through channels, never by calling
  a Qt API directly.
- `QAbstractItemModel` is not thread-safe and is called only from its own thread
  (the GUI thread when a view is attached); background population queues updates
  and applies them on the main thread.

### Object lifetime and ownership

- C++ owns `QObject`s. A Rust struct exposed as a `QObject` is owned by the
  generated C++ object; Rust-side state is accessed through the generated type,
  not through a second allocation.
- Rust must not free a native handle it does not own. Buffers handed to Qt are
  either copied into a Qt-owned container or backed by `Arc`/shared memory with
  an explicit ownership contract.
- No `&mut` aliasing across FFI. Mutable state behind the bridge is accessed
  through the generated API or a lock, never through two live references.
- A Rust panic at the boundary must not unwind into C++. The bridge either aborts
  or converts the panic into an error result; choose and document one.

### Signal/slot from Rust

Proposed shape (CXX-Qt attributes, illustrative only):

```rust
#[cxx_qt::bridge]
mod qobject {
    extern "RustQt" {
        #[qobject]
        type DocumentController = super::DocumentControllerRust;

        #[qsignal]
        fn progress_changed(self: &DocumentController, value: i32);

        #[qslot]
        fn cancel(self: &mut DocumentController);
    }
}
```

The exact attribute set is owned by CXX-Qt and changes between releases. Core
code emits Rust-side events; the `pictura_qt` crate forwards them as Qt signals
on the GUI thread.

### Model exposure

Layers, Channels, and History are `QAbstractItemModel` subclasses implemented in
Rust (via CXX-Qt or a small C++ shim that forwards to Rust data), exposed both to
Widgets views and, if needed, to QML. qmetaobject-rs would support the QML side
directly, but is not chosen because of the Widgets requirement. Role ids and the
`dataChanged` contract are specified in `ARCH-003`.

## Rust module mapping

- `pictura_qt` — the only crate that depends on CXX-Qt. Declares bridges,
  `QObject` subclasses, models, and signal/slot glue.
- `pictura_qt::models` — `QAbstractItemModel` implementations over document
  handles.
- `pictura_qt::threading` — `CxxQtThread` wrappers, queued progress/cancel types.
- `pictura_ffi` — the narrow, Qt-free boundary for buffers and small POD structs
  shared with C++/GPU code.
- `pictura_core` — no Qt dependency; pure Rust document, imaging, and command
  logic. This boundary exists so the core is testable without Qt and survives a
  bridge change.

Data crossing: `u64` node ids, enums as `i32`, strings as `QString` at the Qt
edge, `f32` values, and buffer handles. Never ownership of document images.

## Qt6 component mapping

- `QObject` subclasses generated by CXX-Qt implement the controller layer
  (document commands, tool state, progress).
- `QAbstractItemModel` subclasses back the panels.
- `QMetaObjectConnection` / `ConnectionType` express cross-thread connections
  from the bridge.
- `QRhiWidget` / `QRhiTexture` are owned on the Qt side; Rust receives a handle
  or renders into a shared texture (`ARCH-006`), not a `QImage`.

## Data-model impact

No document fields are added. The bridge carries:

- Commands and their parameters (undo records live in `pictura_core`).
- Progress and cancellation events, which are transient and never serialized.
- Model change notifications, derived from committed commands.

Persistence and PSD/XMP are unaffected.

## Edge cases

- **Panic at the boundary.** Must be caught or aborted deterministically; a panic
  unwinding into C++ is undefined behavior.
- **Last-reference races.** A worker thread holding a buffer handle while the
  document closes. Resolve with reference counting plus a cancellation token, and
  by never letting Qt own Rust memory without an explicit contract.
- **Queued-connection backlog.** A fast producer can flood the GUI thread with
  progress events. Coalesce progress to at most one update per N milliseconds.
- **Linker requirements.** CXX-Qt on Linux requires mold, lld, or ld.gold; the
  build spec must state this (`ARCH-013` build-and-packaging).
- **API churn.** CXX-Qt states early development and frequent API changes. Pin a
  version and isolate the bridge.
- **WASM/other targets.** CXX-Qt tests WASM; the project targets Linux desktop,
  so this is informational only.
- **qmetaobject-rs maintenance.** Passive maintenance is a project risk if that
  path is ever chosen.
- **Qt version skew.** CXX-Qt targets Qt 5.15 LTS and all Qt 6; the build must fix
  a Qt 6 minor version and re-check per release.

## Parity acceptance criteria

- Given a Rust function emits a signal from a worker thread, the connected GUI
  slot runs on the GUI thread and no Qt API is called off-thread.
- Given a `QObject` subclass implemented in Rust is instantiated in C++ and
  destroyed, there is no leak and no double free (validated under valgrind/ASAN).
- Given a Rust panic inside a bridged call, the application terminates or returns
  a defined error, and never unwinds through a C++ frame.
- Given a `QAbstractItemModel` change is produced by a background job, the view
  updates once per committed change and model methods are invoked only on the
  GUI thread.
- Given the `pictura_qt` crate is replaced by a stub, `pictura_core` builds and
  its tests pass without Qt (proves the boundary holds).
- Given a large tile buffer crosses the bridge, no per-tile `QImage` copy occurs
  (measured by copy counters or a profiler).

## Sources

Fetched for this document:

- `https://raw.githubusercontent.com/KDAB/cxx-qt/main/README.md` — CXX-Qt scope,
  crates, maintenance by KDAB, Qt 5.15/Qt 6 support, early-development warning,
  no Rust QWidgets bindings, linker requirements, comparison table (including
  qmetaobject-rs maintenance).
- `https://docs.rs/cxx-qt/latest/cxx_qt/` — crate version 0.10.0, items
  (`CxxQtThread`, `QMetaObjectConnection`, `Threading`, `ConnectionType`,
  `bridge`, `qobject`), dependency on `cxx` and `cxx-qt-build`.
- `https://raw.githubusercontent.com/woboq/qmetaobject-rs/master/README.md` —
  QML-only focus, `cpp!` escape hatch, Qt >= 5.8, passive maintenance and Slint
  direction, `QAbstractListModel` and `QQuickItem` support, missing QWidgets.

Not fetched in this pass:

- `https://crates.io/crates/cxx-qt` — returned a JavaScript-only page; the
  docs.rs page was used instead.

## Open questions

- **CXX-Qt `QAbstractItemModel` coverage.** Whether current `cxx-qt-lib` exposes
  enough of `QAbstractItemModel` (roles, `beginInsertRows`) to implement the
  panel models in Rust, or whether a C++ shim is required. Resolve by reading the
  `cxx-qt-lib` API and prototyping a Layers model.
- **Vulkan device sharing with Qt.** How a Rust (wgpu) device and Qt's QRhi share
  textures and queues. Resolve in `ARCH-006`.
- **Pin policy.** Which CXX-Qt version to pin and the upgrade cadence given the
  early-development warning. Resolve in `ARCH-013` build-and-packaging.
- **Panic strategy.** Abort versus converting to a recoverable error at the
  bridge is unspecified. Resolve with a design note in this spec once chosen.
- **Model role registration to QML.** If QML is later used, role-name exposure
  must be re-checked against the chosen bridge.
- **qmetaobject-rs status change.** If its maintenance resumes, the fallback
  analysis should be revisited.
