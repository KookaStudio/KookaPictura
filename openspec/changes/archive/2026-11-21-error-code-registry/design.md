# Design

## Context

See [proposal.md](proposal.md) for the affected operations. Native Open queues a result to a view and the Qt coordinator finishes a request token. Native Save tracks file publication and document generation. The shared document command applies synchronous engine work to the live document. Filter and adjustment apply move live history into their shared worker. These ownership differences determine where a caught panic can safely become a normal result.

The two draft architecture documents disagree on the capability error. The plug-in ABI assigns it value 2 and reserves value 1 for version mismatch. The error-handling draft assigns capability value 106 among the domain codes.

## Goals / Non-Goals

- Give Rust and Qt one numeric vocabulary, preserve typed causes up to the boundary, and reuse existing operation-specific feedback.
- Protect staged work and existing publication guarantees while retaining worker scheduling and the synchronous entry points.
- Keep the design limited to the proposal's operation paths. The plug-in SDK defines its own buffer ownership with its ABI. This change publishes the shared numeric contract through the first-party bridge.

## Decisions

### Publish one registry and carry status as data

Put the registry, typed error, and guard in a Qt-free `pictura-error` crate. Use explicit `#[repr(u32)]` discriminants for checked Rust codes and a fixed-width raw `u32` for the bridge. Resolve values through a checked lookup so a future number can cross the boundary safely. Keep the plug-in draft's values 1 and 2, reserve 106, and use the `interop/operation-status` table as the contract. Reuse workspace `thiserror` for typed causes. Keep error identity, canonical key, and display detail independent.

Use cxx-qt's existing bridge declarations to deliver the raw code with the operation's typed outcome or signal data. Keep ordinary result payloads, warnings, and operation-specific notices. For covered operations the Qt handlers read the raw code and use existing translated feedback. Legacy callers can continue to use their current success booleans or completion signals through adapters that derive those values from the structured result. Status ownership follows the request or attempt, and a new attempt clears any previous result. The control and MCP protocol strings continue through their existing adapters. Passing status as data handles asynchronous completions and matches the C ABI vocabulary. A bridge exception represents synchronous failure only and complicates Qt event delivery.

### Catch only where state can be safely discarded or classified

Put `catch_unwind` in a Rust guard invoked before the protected staged work can unwind through a bridge or escape the worker that owes completion. Return a typed panic error with a static diagnostic. Ensure disposal of an arbitrary panic payload cannot raise a second unwind at the boundary. Keep unwind enabled in application build profiles and keep the normal panic hook. Use `UnwindSafe` at the guard API and justify any assertion at the call site by identifying the owned or staged state it protects. A C-calling-convention test wrapper can exercise the guard without depending on the plug-in loader.

Handle validation and expected typed failures on their ordinary paths. Treat a worker join panic as a separate failure where a join exists and recoverable state is still available. Limit promises to the protected stages. GUI publication, process aborts, and allocation aborts have their existing crash behavior.

### Finish native Open by request

Carry typed file-access and decode errors through the native loader. Keep native compatibility-view fallback and memory-budget feedback tied to the worker result for that request, including when another open is in flight. On unwind, queue the same terminal callback as on an ordinary failure with an empty staged document and panic status. Route its completion through the coordinator's existing token and teardown rules. Use the token check for duplicate and stale results, and assign cancelled status at the code path that closes the loading tab. Preserve the existing PDF, EPS, raster and raw open control outcomes. Carry request-owned memory refusal through both ControlServer open routes, preserving their protocol codes and cancellation precedence. Retain or replace the process-wide memory query for synchronous recovery callers; do not remove it without migrating those callers.

### Preserve the native save publication phase through unwind

Retain the existing generation and revision checks. Carry a structured operation error alongside the save publication state and the existing origin check. Determine publication from the write path's phase, independent of the error kind. Use unwind-safe temporary-file ownership immediately after exclusive creation, disarm cleanup after successful rename, and retain a rename marker that survives the worker's guarded write. A panic before rename follows the ordinary failed-save path. A panic after rename and before durable acknowledgement follows the uncertain-publication path for the origin. A replaced session cannot inherit the old session's dirty flag, saved state, path, or recovery obligation. Keep the directory sync, action recording, and retry semantics. A native Save a Copy after rename with no durable acknowledgement reports uncertainty with copy-specific feedback: the destination may contain the copy, but the source session and its recovery obligation remain unchanged.

Keep I/O causes typed at the file and stream write boundaries. The native stream sink currently stringifies write and seek failures into codec invalid-data errors. Preserve a typed I/O cause on the native streaming entry point while retaining the existing codec entry points and their display behavior. Classify codec refusals from their typed result and file failures from the I/O source. A filename or a substring in an error message cannot serve as a status discriminator.

### Stage shared whole-document work and worker history

Use the session's document snapshot for the shared synchronous engine path, as in the worker path. Run the engine on the stage and feed both execution modes through the same result handling. Keep the existing live-edit preparation ordering. Capture document generation, revision and history identity for every shared worker, including ordinary document commands, and recheck before publication. A superseded attempt completes as cancelled without failure feedback on its own attempt; it cannot replace a newer attempt's status or result. Carry real preflight outcomes through the Qt command adapter, including refusals before the shared helper and immediate commands. Distinguish invalid arguments from admission refusals and legitimate no-ops, preserving cancellation semantics. Publish the new document and run landing callbacks only for a successful current result. Use the existing edit and presentation path for history capture and invalidation. Retain the current command-size threshold and progress behavior.

Keep an intact history in the live session until the shared filter worker has a result ready to publish. Stage history capture on the worker using shared history pixel storage and an independent set of history bookkeeping. This preserves the off-thread capture requirement and protects the redo branch even if capture panics after it begins. Commit the staged history and document together. On failure, restore presentation from the pre-preview baseline and keep the existing history, selection, dirty flag and last-filter/Fade state. The layer-mask commit path has a separate synchronous handler. The adjustment dialog reports failed OK commits using the operation and structured status, with no panic payload.

## Risks / Trade-offs

- A debug or packaging profile that aborts panics would bypass the guard. Verify ordinary and optimized application test builds use unwind.
- Guarding a save at the worker join alone loses whether rename happened and can leak the temp file. Test faults after the temp file is created and after rename, and verify the destination and recovery behavior through Qt.
- Staging synchronous edits and worker history can add allocation pressure. Share planes, history tiles and existing backing storage, preserve the current capture and progress paths, and check applicable memory and off-thread regressions.
- A new request can supersede a late result. Require the existing request and generation gates before publishing state or delivering status to a live view.

## Migration Plan

Land the registry and protected operation boundaries together with their Qt adapters. Use the existing status text and protocol adapters while migrating each operation. The change has no persisted registry data or file-format migration. Reverting the code restores the previous operation reporting.
