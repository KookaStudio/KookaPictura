# Error Handling

- **Spec ID:** `XC-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — CS6's error surface is the CS5 model: modal error/warning dialogs, a scratch-disk-full error, a "damaged preferences" flow, and (new in CS6) the crash-recovery offer. CS6 adds no documented public error-code registry or structured error API.
- **Depends on:** `01-architecture/rust-core-design.md`, `01-architecture/rust-qt-interop.md`, `01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`), `11-cross-cutting/localization.md` (`XC-001`), `11-cross-cutting/logging-and-telemetry.md` (`XC-003`), `11-cross-cutting/preference-storage.md` (`XC-002`), `11-cross-cutting/security-and-sandboxing.md` (`XC-005`).

> The error taxonomy, FFI error codes, and crate names below are **design proposals**. CS6's internal error codes are closed; the CS6 Help reference documents user-visible symptoms (e.g. the scratch-disk error) but not a code registry. Nothing here claims to reproduce Adobe's codes.

## CS6 behavior

### What the user sees

CS6 errors surface almost entirely as **modal dialogs**. Documented/observed classes (`ARCH-001`, `UI-010`, community):

- **Operation failed:** the generic "Could not complete your request because…" family, sometimes naming the tool or the reason (e.g. "because of a program error", scratch-disk messages).
- **Scratch disk full:** an actionable error; CS6 reports it and stops cleanly rather than corrupting the document (`ARCH-001`).
- **Damaged/corrupt preferences:** at launch Photoshop reports it cannot initialize because the preferences file is invalid, and the fix is to delete/replace it (`XC-002`).
- **GPU/filter unavailable:** a one-time launch warning when the GPU Sniffer fails, and features that disable instead of running wrong (`ARCH-001`).
- **Suppressed warnings:** General → Reset All Warning Dialogs exists because warning dialogs carry a "Don't Show Again" option whose state is persisted.
- **Recovery dialog:** after an unclean exit, CS6 offers recovery of the auto-saved work (CS6 `Automatically Save Recovery Information`; `crash-recovery-and-autosave.md`).
- **Progress/cancel:** long operations show progress and can be cancelled; a cancelled operation must leave the document unchanged (CS6 parity expectation; the exact cancel semantics per tool are not documented).

### A note on Adobe codes

Adobe tooling has used numeric error codes in some products (installation, activation) and some Photoshop troubleshooting threads cite strings like "program error", but no public, stable CS6 *image-pipeline* error-code registry is documented. Kooka Pictura therefore **defines its own** stable codes; claiming Adobe parity for codes is out of scope.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Error/warning dialog | Modal | — | Summary + expandable details; "Don't show again" on some. |
| Scratch-disk error | Modal | — | Actionable (free space / change scratch). |
| Launch GPU warning | Modal | — | One-time. |
| Preferences-damaged notice | Modal | — | Points at the preferences file; reset flow. |
| Recovery prompt | Dialog | — | After an unclean exit. |
| Progress dialog | Modeless | `Esc` | Cancel path must abort cleanly. |
| Status bar / Info | Readout | — | Non-fatal status (scratch sizes, efficiency). |
| `Help > System Info` | Dialog | — | Environment context for support. |
| `Help > Submit Issue` | Menu | — | App extension: opens a prefilled issue with the System Info report. |
| Script error console (proposed, `ARCH-011`) | Panel | — | Script/plug-in tracebacks. |
| "Report / Copy details" (proposed) | Dialog action | — | Crash/defect reporting; opt-in (`XC-003`). |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Severity | enum | — | Info / Warning / Recoverable / Fatal | Drives dialog vs. status vs. recovery. |
| Error code | u32 | — | stable registry | Proposal; never reused (`XC-001` for text). |
| Message key | string | — | catalog key | Localized at display (`XC-001`). |
| "Don't show again" | bool | off | per-dialog ID | `XC-002` suppressed-dialog store. |
| Retry policy | enum | none | none / once / backoff | For device-loss/transient. |
| Panic policy | enum | catch-at-boundary | catch-at-boundary / abort | Keep `unwind` so `catch_unwind` works. |
| Detail disclosure | enum | collapsed | collapsed / expanded | Backtrace/code hidden by default. |
| Report | enum | ask | never / ask / always | Opt-in upload (`XC-003`). |

## Algorithms & pipeline

### The error model across layers

The project follows the standard Rust split, made explicit:

| Layer | Error type | Rationale |
|---|---|---|
| Library crates (`pictura-core`, `-codec`, `-filters`, `-render`, `-color`, `-script`, `-plugin`) | `thiserror` enums, e.g. `pictura_codec::DecodeError` | Typed, matchable, no hidden control flow; `thiserror` produces a plain `std::error::Error` impl and does not appear in the public API surface it decorates. Library errors must stay dependency-light. |
| Service / command layer | typed enums or `Result<T, pictura_error::Error>` | One crate-level error where an operation spans libraries. |
| FFI boundary (Rust ↔ Qt/C plug-ins) | `#[repr(C)]` `OpStatus` code + host-owned UTF-8 message | `Result`/trait objects cannot cross a stable ABI; codes are the API. |
| Application boundary (`pictura-app`) | `anyhow::Result<T>` | `?`-friendly, attaches context/backtraces, downcastable to typed errors at the edge. `anyhow` is **not** used in library public APIs. |

- **`thiserror` in libraries.** Errors are enums/structs deriving `Error`; `#[from]`/`#[source]` preserve the cause chain; `#[error(transparent)]` forwards an opaque inner type when hiding implementation details is desired. Public enums are `#[non_exhaustive]` so a new variant is not a breaking change.
- **`anyhow` at the boundary.** `pictura-app` and CLI entry points return `anyhow::Result`; `.context()`/`.with_context()` add the operation ("Failed to save <n>"), and `anyhow::Error::downcast_ref` recovers the typed code for the code-stability contract. Backtraces follow `RUST_BACKTRACE`/`RUST_LIB_BACKTRACE` (Rust ≥ 1.65 captures a backtrace on demand).
- **Beginner rule:** libraries never `anyhow!` and never panic on user input; `unreachable!`/`expect` are reserved for true invariants and are reviewed.

### FFI status codes

Mirror the plug-in ABI's approach (`ARCH-011`) so plug-ins, scripts, and the Qt bridge share one vocabulary:

```text
/* Stable across releases; append-only. Codes are the public contract. */
enum OpStatus {
  OP_OK              = 0,
  OP_ERR_INVALID_ARG = 3,
  OP_ERR_CANCELLED   = 4,
  OP_ERR_HOST        = 5,   /* internal */
  OP_ERR_PANIC       = 6,
  /* domain codes (stable, append-only) */
  OP_ERR_CODEC       = 100, OP_ERR_IO        = 101,
  OP_ERR_COLOR       = 102, OP_ERR_GPU       = 103,
  OP_ERR_SCRATCH     = 104, OP_ERR_SCRIPT    = 105,
  OP_ERR_CAPABILITY  = 106, OP_ERR_LICENSE   = 107
}
```

Rules: codes are never renumbered or reused; a code has a single canonical message key (`XC-001`); a `#[repr(C)] OpError { code: u32, message: OpStr }` crosses the boundary with a host-owned buffer; the Qt side maps `(code, key, args)` to a localized `UiError`. cxx-qt can also surface a Rust `Result` as a C++ exception at the bridge; the design picks **one** mechanism per boundary (codes for the C plug-in ABI; cxx-qt `Result`/exception for first-party Qt calls) rather than mixing silently.

### Panic safety

- **At every FFI boundary**, exported functions wrap their body in `std::panic::catch_unwind` and translate a caught panic to `OP_ERR_PANIC`; the host survives and disables/reports the offending plug-in (`ARCH-011` parity acceptance).
- `catch_unwind` catches **unwinding** panics only; a panic implemented as abort (or a foreign C++ exception crossing a non-`C-unwind` boundary) is not caught. The GUI binary therefore keeps `panic = "unwind"` so boundary catching is meaningful. `extern "C"` functions that panic without catching abort the process by definition; the project does not rely on that.
- The panic hook (`XC-003`) records location/message/backtrace to the crash log before the unwind is caught or the process aborts.
- `UnwindSafe`/`AssertUnwindSafe` are used deliberately; `AssertUnwindSafe` is justified in review, not sprinkled.
- Rayon worker panics propagate through `join`; the command layer converts a worker panic into a command failure that **aborts the transaction** so the document is unchanged.

### Recovery vs. crash

```text
error occurs
  ├─ user mistake / validation        → Recoverable: dialog, document unchanged, no history entry
  ├─ recoverable I/O / GPU device loss → Recovery path: retry or degrade (CPU fallback), WARN, keep doc
  ├─ out of memory / scratch full     → Fatal for the operation: stop cleanly, surface actionable path,
  │                                     do NOT corrupt; offer save/close
  └─ process panic/crash              → crash record + autosave recovery on next launch (XC-003)
```

Transactional invariant: **a failed command leaves the document byte-identical to before the command and adds no history state** (asserted by `ARCH-011` criterion 10 for plug-ins, generalized here). The undo layer brackets commands; any error path calls `undo_abort` rather than `undo_commit`.

### User-facing presentation

- Map severity → surface: Info → status/log only; Warning → non-modal or modal warning (respect "Don't show again"); Recoverable → dialog naming the operation and offering retry/choices; Fatal → dialog + clean stop.
- Show a **human summary** (localized key) and keep the **code + technical detail** behind a disclosure (copyable). Never show a raw Rust `Debug` string as the primary message.
- Offer "Copy details" (code, key, context, versions, log excerpt) and, when enabled, "Report" (opt-in; `XC-003`).
- A message may contain a because-clause, but it must name the operation and the next action; a generic program-error fallback is a last resort.

## Rust module mapping

- `pictura-error::ErrorCode` — `#[repr(u32)]` stable codes; `#[non_exhaustive]`; `Display` maps to a message key, not a sentence.
- `pictura-error::Error` — the cross-crate typed error (thiserror) with `#[source]` chain and `code()`.
- `pictura-error::Severity` — `Info | Warning | Recoverable | Fatal`.
- `pictura-error::UiError` — `{ code, message_key, args, detail }` ready for the Qt layer.
- `pictura-error::ffi` — `guard(f) -> Result<T, OpError>` wrapping `catch_unwind`; `OpError`, `OpStr`; the `OpStatus` constant table.
- `pictura-error::Context` — small helper for `anyhow` context at boundaries.
- Per-crate errors live with their crate (`pictura_codec::DecodeError`, …) and convert `Into<pictura_error::Error>`.
- Types crossing Rust↔Qt: `UiError`, `ErrorCode`, `Severity`, `OpError`. No `anyhow::Error`/`thiserror` types cross the boundary.

## Qt6 component mapping

- `ErrorDialog` (`QDialog`) — summary, code, expandable detail, Copy/Report; built from `UiError`.
- `QMessageBox` — quick warning/info; use a custom dialog when detail/copy is needed.
- `QErrorMessage` — optional dedup for repeated non-modal messages; may not fit the dark theme.
- `ProgressDialog` (`QProgressDialog`) — cancel maps to `OP_ERR_CANCELLED`; cancel must abort the transaction.
- cxx-qt bridge — converts Rust typed errors to C++ exceptions or `Result` for first-party calls; exceptions from the bridge are caught in the slot and turned into `UiError`.
- `QGuiApplication::notify` override (optional last-resort) — catch exceptions escaping Qt event handling, log, show a fatal dialog, avoid `std::terminate` where reasonable.
- `qWarning`/`qCritical` — for unexpected states; routed into the shared log sink (`XC-003`).

## Data-model impact

- **No persistence of errors** in the document. An error must not mutate the document model; failed commands roll back.
- **History:** a failed/cancelled command adds no history state; a partially completed multi-step command aborts as a unit (`undo_abort`).
- **Suppressed-dialog state** is stored per stable dialog ID, not message text (`XC-002`, `XC-001`).
- **Crash/recovery records** are application state, not document data (`XC-003`; `crash-recovery-and-autosave.md`).
- **Actionable error payloads** (e.g. scratch path, GPU name) are derived at display time and not serialized into the file.
- **Script/plug-in errors** carry an opaque message + code; they are reported and logged, never written into PSD/XMP (`ARCH-011`).

## Edge cases

- **Panic across FFI:** undefined behavior unless caught; every exported function guards with `catch_unwind`; a caught panic becomes `OP_ERR_PANIC`.
- **Double panic / panic while unwinding:** aborts; the panic hook still runs; document the hard limit.
- **Panic implemented as abort:** not catchable; process dies; crash record only if the hook ran before abort (it may not).
- **OOM / allocation failure:** Rust aborts on allocation failure by default; treat OOM as fatal, rely on autosave, and never try to "handle" it in-process.
- **Poisoned mutex:** prefer lock-free/`Mutex::lock` error handling; a poisoned lock is a bug → fatal for the operation, not a user error.
- **GPU device loss:** recover by re-creating the device and re-uploading visible tiles (`ARCH-001`); surface a WARN, not a fatal dialog.
- **Scratch full:** actionable error, stop cleanly, do not corrupt (`ARCH-001`).
- **Partial multi-tile failure:** abort the whole command; never commit a half-filtered layer.
- **Cancel vs. error:** cancel is not an error; no dialog, no log at ERROR, no history entry.
- **Error during save:** atomic write semantics (`QSaveFile`/temp+rename) keep the original file intact; surface the failure.
- **Localization:** message text comes from the catalog; the code is locale-invariant (`XC-001`).
- **Error-code stability:** adding a code is fine; renumbering is forbidden and must be tested.
- **Plug-in error strings:** are untrusted text; escape/limit length before display and never render as rich text (injection; `XC-005`).
- **Infinite retry:** bounded retry/backoff; a transient-failure loop must not hang the GUI.
- **Error inside the error handler:** the dialog/log path is defensive and must not itself panic; keep it allocation-light.

## Parity acceptance criteria

1. Given a filter that fails mid-operation, a dialog names the operation and the document is byte-identical to before the command, with no new history state.
2. Given a plug-in whose exported entry panics, the host reports `OP_ERR_PANIC`, disables the plug-in, and the document and host remain usable (`ARCH-011` criterion 4).
3. Given the GPU device is lost during editing, the app re-creates the device, re-uploads visible tiles, and continues (WARN logged), without a fatal dialog and without data loss.
4. Given a scratch disk fills during a save/operation, the app stops that operation cleanly, keeps the original file intact, and shows an actionable scratch error.
5. Given the user cancels a long operation, no error dialog appears, no history state is added, and the document is unchanged.
6. Given a corrupt preferences file, the app starts with defaults and a notice, and does not crash (`XC-002`).
7. Given an error surfaced in a non-English locale, the summary uses the localized catalog text while the numeric code is unchanged across locales.
8. Given an error code from version N, version N+1 maps it to the same condition; a test asserts no code is reused or renumbered.
9. Given a script/plug-in returns an oversized or markup-laden error string, the dialog shows it as plain, length-limited text with no markup interpretation.
10. Given `Don't show again` was set for a warning, it stays suppressed across restarts until `Reset All Warning Dialogs` (`XC-002`).

## Sources

Fetched for this document:

- `https://docs.rs/thiserror/latest/thiserror/` — `thiserror` 2.0 derive for `std::error::Error`; `#[from]`/`#[source]`/`#[backtrace]`; `#[error(transparent)]` for opaque inner errors and stable public error types; not present in the decorated public API.
- `https://docs.rs/anyhow/latest/anyhow/` — `anyhow::Error`, `Result<T>`, `?`, `context`/`with_context`, downcasting (by value/ref/mut), `anyhow!`/`bail!`/`ensure!`, backtrace capture (Rust ≥ 1.65) via `RUST_BACKTRACE`/`RUST_LIB_BACKTRACE`, explicit "use this in application code" positioning.
- `https://doc.rust-lang.org/std/panic/fn.catch_unwind.html` — `catch_unwind` catches **unwinding** panics only; `extern "C"` functions that panic abort; `UnwindSafe`/`AssertUnwindSafe`; dropping an `Err` payload may panic; not a general try/catch.
- `https://doc.rust-lang.org/std/panic/index.html` — panic module: `set_hook`/`take_hook`, `catch_unwind`, `resume_unwind`, `panic_any`, `UnwindSafe`/`RefUnwindSafe`, `PanicHookInfo`.
- `https://doc.qt.io/qt-6/qloggingcategory.html` — fatal messages always enabled; the default message handler aborts to create a core dump (relevant to "fatal" mapping and diagnostics).
- `https://doc.qt.io/qt-6/qsavefile.html` — atomic save behavior that protects the existing file when a save fails; `cancelWriting()`; `setDirectWriteFallback` caveat.

Found via search, **not fetched** (leads):

- Adobe Community threads on CS6 "Could not complete your request because…" and "Could not initialize Photoshop because the preferences file…" — search-result evidence only; used to characterize the CS6 modal-error surface, not as an authoritative code list.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 reference; not re-fetched. Scratch-disk and preferences-damage behavior is cited via `ARCH-001`/`UI-010`.

Internal cross-references (not sources): `docs/01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`) for `OpStatus`/`catch_unwind`/transaction-abort semantics; `docs/01-architecture/system-architecture.md` (`ARCH-001`) for scratch/GPU recovery.

## Open questions

- **Did CS6 expose any stable numeric error codes to the user or to scripts?** No public registry found. *Resolve:* inspect CS6 dialogs/ExtendScript error objects or archived SDK docs; otherwise Kooka Pictura codes are its own and parity is behavioral only.
- **Exact CS6 cancel semantics per operation.** Whether every long operation is cancellable and whether cancel rolls back atomically is not documented. *Resolve:* toggle-and-observe on CS6.
- **Does cxx-qt map Rust `Result` to C++ exceptions, and do Qt slots catch them reliably?** *Resolve:* a bridge spike in `rust-qt-interop.md`.
- **`panic = "abort"` vs `"unwind"` for the final binary.** Unwind is required for boundary `catch_unwind`; abort is simpler/smaller. *Resolve:* decide with the plug-in ABI and perf budget.
- **OOM policy.** Whether to attempt graceful OOM handling (futures/fallible allocation) or rely on abort + autosave. *Resolve:* `ARCH-003`/memory strategy.
- **Whether an error-code registry ships in `pictura-error` or as generated data** (so docs/UI/tests share one source). *Resolve:* codegen prototype.
- **Recovery UX ownership** — this doc assumes `crash-recovery-and-autosave.md` owns the actual snapshot/restore. *Resolve:* cross-spec review.
