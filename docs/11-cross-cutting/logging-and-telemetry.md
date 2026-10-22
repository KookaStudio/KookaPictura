# Logging and Telemetry

- **Spec ID:** `XC-003`
- **Status:** `Draft`
- **Parity tier:** `Core` (user-facing History Log and crash reporting); privacy-preserving telemetry is a **Core** behavior modeled on CS6's opt-in Adobe Product Improvement Program; GPU/performance diagnostics are `Core`.
- **New in CS6:** `Changed` — CS6 keeps the History Log (General preferences) from CS5 and the opt-in Adobe Product Improvement Program; CS6 adds no documented application-wide diagnostic log file, but adds the crash-recovery/auto-save flow and the expanded GPU settings that a diagnostics system must describe. Telemetry as a *first-class, inspectable* feature is a Kooka Pictura addition.
- **Depends on:** `02-ui-ux/preferences.md` (`UI-010`), `01-architecture/system-architecture.md` (`ARCH-001`), `01-architecture/performance-targets.md`, `01-architecture/gpu-rendering-pipeline.md`, `11-cross-cutting/error-handling.md` (`XC-004`), `11-cross-cutting/crash-recovery-and-autosave.md`, `11-cross-cutting/preference-storage.md` (`XC-002`), `11-cross-cutting/security-and-sandboxing.md` (`XC-005`).

> Rust crate names and log/telemetry schemas below are **design proposals**. CS6's diagnostic logging internals are not publicly documented; where the exact CS6 behavior is unknown it is marked *(unverified)*.

## CS6 behavior

### User-facing edit history: the History Log

The one logging facility CS6 documents is the **History Log**, configured under `Edit > Preferences > General` (`UI-010`):

- **Off by default**; when enabled it records edit history.
- **Save Log Items To**: Metadata (writes into the document's metadata, i.e. XMP), Text File (a user-chosen path), or Both.
- **Edit Log Items**: Sessions Only, Concise, or Detailed.
- It records *user edits* (commands), not application diagnostics; it is a provenance/audit feature, not a debug log.

This is the parity contract: Kooka Pictura must offer the same History Log behavior (metadata/text/both, three verbosity levels) and must keep the History Log distinct from any diagnostic logging.

### Adobe Product Improvement Program (APIP)

CS6-era Creative Suite ships the **Adobe Product Improvement Program**. Per Adobe's documentation:

- After the software has been used a certain number of times, a dialog asks whether the user wants to participate.
- Participation is **optional** and **opt-in**.
- If the user participates, usage data about the Adobe software is sent to Adobe; no personal information is recorded or sent. The Adobe Product Improvement Program collects only which features and tools the user works with and how often. *(Adobe Help, robohelp 2015 edition)*
- The user can opt in/out at any time via `File > Help > Adobe Product Improvement Program` → "Yes, Participate" / "No, Thank You".
- Adobe describes the program as designed to scale to millions of users without disrupting their product use, with the data sent automatically only after a period (Adobe privacy page) — i.e. it is batched, not a live stream.

Whether the exact same program shipped in every CS6 application, and its precise event schema for Photoshop CS6, is **not established here**. Kooka Pictura should treat APIP as the *behavioral* precedent: **opt-in, anonymous, usage-frequency only, revocable, no content**.

### Diagnostics and performance

- CS6 has no documented end-user diagnostic log file comparable to a modern structured log. Troubleshooting is via the preferences file, `Help > System Info` (environment report), and the status bar.
- **Status bar** exposes `SCRATCH SIZES` and `EFFICIENCY` — Efficiency is the percentage of time computing rather than paging, and sustained values below ~95% indicate scratch-disk paging (`ARCH-001`). This is a live performance diagnostic.
- **`Help > System Info`** reports the environment and detected GPU. VRAM is
  `Unavailable` because wgpu does not expose it.
- **`Help > Submit Issue`** (app extension) copies the report to the clipboard
  and opens the repository's new-issue page prefilled with it.
- **GPU Sniffer** probes the GPU at launch and disables GPU features on failure (`ARCH-001`), producing a launch-time warning dialog.
- Adobe's crash reporter uploads crash data; the exact CS6 mechanism and consent text are *(unverified)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Preferences > General > History Log` | Preference | — | Off / on; Metadata / Text File / Both; Sessions/Concise/Detailed. |
| `File > Help > Adobe Product Improvement Program` | Menu | — | APIP opt-in/out (CS6-era). |
| `Help > System Info` | Dialog | — | Environment and GPU report. |
| `Help > Submit Issue` | Menu | — | App extension: opens a prefilled issue with the System Info report. |
| Status bar | Readout | — | `SCRATCH SIZES`, `EFFICIENCY`. |
| Launch GPU warning | Modal | — | One-time GPU-sniffer failure notice. |
| Crash/recovery prompt | Dialog | — | Offer recovery after a crash (`crash-recovery-and-autosave.md`). |
| `Help > Open Log Folder` (proposed) | Menu | — | Reveal the log directory; not in CS6. |
| `Preferences > Privacy` / telemetry consent (proposed) | Preference | — | Opt-in toggle, view payload, reset ID; not in CS6. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| History Log | toggle | off | on / off | CS6 parity. |
| History Log destination | enum | Metadata | Metadata / Text File / Both | CS6 parity. |
| History Log verbosity | enum | Sessions Only | Sessions Only / Concise / Detailed | CS6 parity. |
| Console log level | enum | Info | Off/Error/Warn/Info/Debug/Trace | Proposal; per-target filters. |
| Log filter rules | string | empty | `target=level` rules | Proposal; mirrors `QT_LOGGING_RULES`. |
| File log rotation | enum | Daily | never/minute/hour/day | `tracing-appender`. |
| Files retained | int | 7 | 1–n | Proposal. |
| Log directory | path | `$XDG_STATE_HOME/kooka-pictura/logs` | any writable | Proposal. |
| Telemetry | toggle | off | on / off | Opt-in; CS6 APIP default is also opt-in. |
| Telemetry endpoint | URL | (none shipped) | HTTPS | Must be documented; no silent default. |
| Sample rate | float | 1.0 | 0–1 | For high-volume events. |
| Install ID | UUID | generated on consent | resettable | Anonymous; not tied to license. |
| Crash report | enum | ask | never / ask / always | Proposal; "always" still shows the payload. |

## Algorithms & pipeline

### Logging architecture (proposed)

Use **`tracing`** as the instrumentation API for first-party Rust. `tracing` supports structured *spans* (bounded time) and *events* (points in time) with typed fields, which suits tile composites, filter kernels, and GPU work far better than free-text logs. Library crates depend only on `tracing`; the application binary installs a subscriber. Third-party crates that use the `log` facade are captured through `tracing`'s `log` feature or `tracing-log`, so there is one sink.

- **Sink:** `tracing-subscriber` with an `EnvFilter`-style layer set (per-target/level), plus a formatted console layer when run from a terminal and a file layer for support bundles.
- **File output:** `tracing-appender` `RollingFileAppender` (daily rotation) wrapped in its non-blocking writer so logging never stalls the GUI or a render thread; the returned `WorkerGuard` must outlive the worker so buffered logs flush on abrupt termination.
- **System integration:** optionally mirror WARN+ to the systemd journal (`systemd-journal-logger`) or syslog; Flatpak apps should log to the journal/stderr and keep their own files in the sandboxed state dir.
- **Qt side:** Qt's own logging goes through `QLoggingCategory` and a message handler. Install a `qInstallMessageHandler` that forwards `QtMsgType` records into the same `tracing` subscriber (mapping Debug/Info/Warning/Critical/Fatal to the corresponding levels), so Qt and Rust interleave in one ordered log. Category rules (`QT_LOGGING_RULES`/`qtlogging.ini`) remain available for Qt internals.
- **Levels:** ERROR (user-affecting failure), WARN (degraded/recoverable), INFO (lifecycle: open/save/GPU init), DEBUG (per-command/tile, off by default), TRACE (per-pixel/per-frame; off by default and compiled out in release via `release_max_level_*` if desired).
- **Redaction:** a formatter layer or `tracing` field visitors strip document paths, filenames, and metadata before a log line reaches disk or a support bundle. Errors log **message keys and numeric error codes** (`XC-004`), not user content.

### Performance and GPU diagnostics

- Instrument hot paths with spans: `document.open`, `codec.decode`, `render.composite`, `filter.<name>`, `gpu.upload`, `tile.<id>`. Record tile counts, byte counts, and elapsed time.
- Expose a **Diagnostics panel** (proposed) that renders: frame-time histogram, tile-cache hit rate, peak RAM/VRAM, scratch usage, decoded-format timings, and the GPU Sniffer result. CS6's nearest equivalents are the status-bar Efficiency readout and System Info.
- `ARCH-001`'s Efficiency metric (compute vs. paging) is reproduced as a counter and surfaced the same way.
- GPU device-loss and fallback events are logged at WARN and included in support bundles.

### Crash logs

- Install `std::panic::set_hook` to capture panic location/message/backtrace before unwinding; write a crash record (JSON) to `$XDG_STATE_HOME/kooka-pictura/crashes/`. `catch_unwind` at FFI boundaries (`XC-004`) already prevents most panics from killing the process, so the hook is a last resort plus a first-class crash record.
- On the next launch, if an unclean shutdown/crash record exists, offer `crash-recovery-and-autosave.md` recovery and a "report this crash" action.
- Crash reporting is **opt-in and explicit**: show the exact payload, allow redaction, never auto-upload. Distinguish the application's crash record from an OS core dump.
- Note: a panic implemented as `abort` (e.g. from a C library or a `panic = "abort"` build) is not caught and may not run the hook; document this limitation and keep `unwind` for the GUI binary so boundary `catch_unwind` works (`XC-004`).

### Telemetry policy (opt-in, privacy-preserving)

Design constraints, modeled on APIP but stricter:

1. **Off by default.** No network activity until the user consents (or a policy explicitly enables it).
2. **Anonymous.** A random install ID stored in the state dir; no license key, account, or hardware fingerprint. A one-click "reset ID".
3. **Content-free.** Never collect image pixels, document contents, filenames, file paths, layer names, XMP/EXIF, clipboard, or window titles. Only event names, counters, coarse timings, application version, OS/arch, and Qt/Rust versions.
4. **Batched and rate-limited.** Queue events locally; send on a schedule with backoff; cap queue size.
5. **Inspectable.** A UI page lists queued events and the exact JSON payload; export/clear.
6. **Revocable.** Turning telemetry off stops collection and deletes the queue; the previously sent data is not recalled but no further data leaves.
7. **Kill switch.** Honor a policy/preference and an environment variable (`KOOKA_PICTURA_NO_TELEMETRY=1`) that disables telemetry regardless of UI state; respect enterprise config under `/etc/xdg` (`XC-002`).
8. **Transport security.** HTTPS only; pin/reject invalid certs; no plaintext. A Flatpak build without network permission must degrade to no telemetry rather than request broad `--share=network` solely for it (`XC-005`).
9. **Transparency.** Document the endpoint, schema, and retention in the repo and in-app.
10. **No third-party trackers.** No embedded analytics SDKs by default; the endpoint is project-owned or clearly disclosed.

Sample event schema (proposal):

```json
{
  "event": "command.executed",
  "ts": "2026-09-14T12:00:00Z",
  "app": { "version": "0.1.0", "channel": "stable" },
  "env": { "os": "linux", "arch": "x86_64", "qt": "6.8.2", "gpu_class": "discrete" },
  "install_id": "9f2c…",
  "props": { "command_id": "filter.gaussian_blur", "depth": 8, "mode": "rgb" }
}
```

Note the absences: no path, no filename, no image dimensions that could identify a document, no user text.

### Pipeline

```text
tracing events/spans (Rust)  ─┐
log-facade events (deps)     ─┤→ tracing-subscriber (EnvFilter)
QLoggingCategory (Qt)        ─┘        │
                                       ├─ console layer  (stderr, dev)
                                       ├─ rolling file layer → $XDG_STATE_HOME/kooka-pictura/logs/
                                       ├─ journald/syslog layer (optional)
                                       └─ metrics bridge → Diagnostics panel

panic hook → crashes/*.json → next-launch recovery + opt-in report
consented events → local spool (bounded) → HTTPS batch → endpoint (never content)
```

## Rust module mapping

- `pictura-log::init(LogConfig)` — installs the `tracing-subscriber` layer stack; idempotent; returns a `WorkerGuard` that must be kept alive.
- `pictura-log::LogConfig` — level, per-target filters, file rotation/retention, journald toggle, redaction toggle; loaded from `XC-002` prefs.
- `pictura-log::redact` — field visitor/formatter that removes paths/filenames/PII.
- `pictura-log::crash` — `set_hook` installer; `CrashRecord { panic, backtrace, app, env, timestamp }`; writer/reader.
- `pictura-log::metrics` — spans/counters for performance; feeds the Diagnostics panel; never crosses the network by itself.
- `pictura-telemetry::Telemetry` trait — `record(Event)`, `flush()`, `enabled()`; a `NoopTelemetry` default and a `HttpTelemetry` implementation.
- `pictura-telemetry::Event`, `Envelope`, `InstallId` (UUID), `Spool` (bounded local queue), `Consent`.
- `pictura-cli` / `pictura-app` — owns subscriber init; libraries must **not** call `set_global_default` (`tracing` guidance).
- Types crossing the Rust↔Qt boundary: `LogConfig`, `CrashRecord` (summary), `TelemetryConsent`, queued-event summaries.

## Qt6 component mapping

- `QLoggingCategory` + `Q_DECLARE_LOGGING_CATEGORY` — Qt-side categories (e.g. `kookapictura.ui`, `kookapictura.bridge`); category rules via `QT_LOGGING_RULES`/`qtlogging.ini`.
- `qInstallMessageHandler` — forward `QtMsgType` to the Rust sink; consider `qFormatLogMessage`/`qSetMessagePattern` for console.
- `SystemInfoDialog` — environment/GPU/session report (`Help > System Info`).
- `DiagnosticsPanel` / `DiagnosticsController` (`QObject`, cxx-qt) — live performance/GPU readouts.
- `CrashDialog` — shows the crash record and the "report" choice.
- `TelemetryConsentDialog` + `PrivacyPreferencesPage` — opt-in, payload viewer, reset ID, endpoint disclosure.
- `HistoryLogPreferences` — the CS6 History Log controls (`UI-010`).

Widgets (not QML) are proposed for these dialogs, matching the CS6 native-dialog feel and accessibility (`UI-012`).

## Data-model impact

- **History Log is document-adjacent, not app-log.** When destination is Metadata, the edit log is written into the document's XMP at save time (a document-model/serialization concern), not into the application log. Text File destination writes a separate user-chosen file. The three verbosity levels map to fixed record shapes in the document/command layer.
- **Application logs, crash records, and telemetry live outside the document** and never serialize into PSD/XMP.
- **New persisted state:** telemetry consent + install ID + spool in `$XDG_STATE_HOME`; log configuration in `prefs.toml` (`XC-002`).
- **No secrets** are stored; the install ID is a random identifier with no key material (`XC-005`).
- **Undo:** neither logging nor telemetry creates document undo entries.

## Edge cases

- **Log directory unwritable / disk full:** fall back to stderr/journald; drop log lines rather than block or crash; warn once.
- **Log flood:** rate-limit a misbehaving target; drop DEBUG/TRACE above a per-second budget.
- **Panic during logging / reentrancy:** the panic hook and the log subscriber must not allocate-path into each other; guard against recursive logging.
- **Panic implemented as abort:** not catchable; documented; keep unwind for boundary catching.
- **Logging across FFI:** the plug-in ABI routes logs through a host callback (`ARCH-011`); the host must apply the same redaction.
- **Telemetry without network / Flatpak no-network:** spool then drop after a bounded age; never retry forever; never request network permission just for telemetry.
- **Consent withdrawn mid-session:** stop immediately and delete the spool.
- **PII leaks:** filenames, document titles, and layer names are the classic leak; redaction tests must assert they never appear.
- **Clock/timezone:** all timestamps UTC/ISO-8601; logs sortable across locales.
- **Huge backtraces / deep recursion:** cap backtrace frames captured into crash records.
- **Multiple instances:** each writes its own log stream or interleaves safely; the appender must not corrupt files.
- **Support bundles (proposed):** an explicit "Export diagnostics" action that assembles logs + System Info + crash records with redaction preview.
- **History Log growth:** metadata log can bloat PSD/XMP; cap or warn per CS6 behavior.

## Parity acceptance criteria

1. Given `History Log = On` and destination `Metadata`, editing a document and saving embeds the log in the document metadata with the chosen verbosity; with destination `Text File`, the chosen file contains it; `Both` produces both.
2. Given `History Log = Off`, no log is recorded anywhere.
3. Given the app launched with `LANG` set and run from a terminal, logs appear on stderr at the configured level with UTC timestamps; setting the level to `Debug` reveals per-command/tile spans.
4. Given a log directory that is read-only, the app continues, logs to stderr, and warns once without crashing.
5. Given a deliberate panic in a worker thread, a crash record with backtrace is written to the state dir and the next launch offers recovery and an opt-in report.
6. Given telemetry has never been consented to, no network request is made by the telemetry subsystem (verified by packet capture / a stub sink).
7. Given telemetry consented, the queued payload contains only the documented fields; a test asserts no path/filename/document-content field can be serialized.
8. Given telemetry is turned off, the spool is deleted and no further requests occur; given `KOOKA_PICTURA_NO_TELEMETRY=1`, no requests occur even if the UI toggle is on.
9. Given a Flatpak build with no `--share=network`, telemetry is disabled and the app still runs.
10. Given `Help > System Info`, the dialog reports the environment and detected GPU (VRAM is `Unavailable`); the Diagnostics panel reports frame time and tile-cache hit rate consistent with `ARCH-001`.
11. Given the History Log is enabled, it produces no application-log entries and vice versa.

## Sources

Fetched for this document:

- `https://docs.rs/tracing/latest/tracing/` — structured spans/events/subscribers; `#[instrument]`; field recording and `%`/`?` sigils; the `log` feature and `tracing-log` interop; libraries should not call `set_global_default`; `rustc` 1.65+.
- `https://docs.rs/log/latest/log/` — `log` facade; five levels (error/warn/info/debug/trace); target/level filtering; `set_logger`/`set_max_level`; compile-time `max_level_*`/`release_max_level_*`; a list of sink implementations including `env_logger`, `systemd-journal-logger`, `syslog`, and `tracing` compatibility.
- `https://docs.rs/tracing-appender/latest/tracing_appender/` — rolling file appender (`Rotation::MINUTELY/HOURLY/DAILY/NEVER`), non-blocking writer, `WorkerGuard` flushes buffered logs on abrupt termination.
- `https://doc.qt.io/qt-6/qloggingcategory.html` — `QLoggingCategory` / `Q_DECLARE_LOGGING_CATEGORY` / `qCDebug`…`qCFatal`; rule syntax; `QT_LOGGING_RULES` / `QT_LOGGING_CONF` / `qtlogging.ini`; `installFilter`; `QtFatalMsg` always enabled; fatal default handler aborts to create a core dump.
- `https://help.adobe.com/en_US/robohelp/2015/robohtml/book/rob_gettingstarted_gs/Adobe_Product_Improvement_Program-.htm` — Adobe's own APIP description: a prompt appears after a number of uses, participation is optional/opt-in, no personal information is recorded or sent, and only the features/tools used and their usage frequency are collected, with opt-in/out at `File > Help > Adobe Product Improvement Program`. (Adobe Help, 2015 edition **but describes the APIP of that era, including CS6-era Creative Suite**; not Photoshop-CS6-specific.)
- `https://rustsec.org/` — RustSec Advisory Database and `cargo-audit`/`cargo-deny` tooling; used to argue for supply-chain auditing of the logging/telemetry dependency tree.
- `https://specifications.freedesktop.org/basedir-spec/latest/` — `$XDG_STATE_HOME` is explicitly for "state data that should persist between (application) restarts" such as "actions history (logs, history, recently used files)".

Found via search, **not fetched** (leads):

- `https://www.adobe.com/privacy/apip.html` — request timed out. Adobe's APIP privacy page; the robohelp page above is the fetched substitute.
- `https://helpx.adobe.com/photoshop/kb/preference-file-names-locations-photoshop.html` — HTTP 403.

Internal cross-references (not sources): `docs/02-ui-ux/preferences.md` (`UI-010`) for the History Log controls; `docs/01-architecture/system-architecture.md` (`ARCH-001`) for Efficiency/GPU Sniffer/System Info; `docs/01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`) for host log callbacks.

## Open questions

- **Exact CS6 APIP behavior for the CS6 timeframe and for Photoshop specifically.** The fetched Adobe page is a 2015 RoboHelp general description; the CS6 launch announcement mentions APIP but the CS6 schema/consent text is not established. *Resolve:* archived Adobe "APIP FAQ" (`adobe.com/misc/apipfaq.html`) via the Wayback Machine.
- **Does CS6 write any diagnostic log file at all** (e.g. a verbose/troubleshooting mode)? *(unverified)*. *Resolve:* inspect a CS6 install/registry for a logging flag.
- **CS6 History Log metadata format.** The exact XMP/metadata keys the History Log writes are not established. *Resolve:* enable it in a CS6 install and inspect a saved PSD's XMP.
- **Crash reporter consent and transport in CS6** — unknown. *Resolve:* CS6 release notes / crash-handler docs.
- **Telemetry endpoint ownership and consent UX.** Whether Kooka Pictura ships any endpoint at 1.0, and if so who operates it, is a product/legal decision. *Resolve:* project decision + `00-overview/licensing-and-provenance.md`.
- **`tracing` vs `log` as the public instrumentation API** for third-party plug-ins; whether plug-ins get a tracing handle or a narrow host `log` callback only. *Resolve:* align with `ARCH-011`.
- **Log retention/rotation defaults and support-bundle contents.** *Resolve:* support/usability review.
- **Performance budget for always-on spans.** Whether INFO-level instrumentation measurably affects frame budget; verify against `ARCH-003`/`performance-targets.md`.
