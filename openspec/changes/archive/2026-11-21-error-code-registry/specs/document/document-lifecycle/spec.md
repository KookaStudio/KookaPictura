# Native Open and Save Operation Status

## ADDED Requirements

### Requirement: Native Open reports a terminal operation status

A native PSD/PSB loading-tab request SHALL carry its status on completion. The request SHALL distinguish successful open, malformed or unsupported native data, file-access failure, and an unwinding panic in the guarded open work. The completion and memory-budget refusal reason SHALL be tied to the request that produced them. Both ControlServer open routes SHALL use that request-owned reason while preserving existing protocol codes and cancellation precedence. User cancellation SHALL complete once as `OP_ERR_CANCELLED`. A later result from that request SHALL be discarded.

#### Scenario: Native Open classifies per-request I/O and codec failures [os_open_error_kinds]

- **WHEN** a malformed PSD and a different PSD that becomes unreadable after routing are opened concurrently
- **THEN** the malformed request completes with `OP_ERR_CODEC` and the unreadable request completes with `OP_ERR_IO`, each reported against its own file with no loading tab or document left for either request

#### Scenario: A panic completes a native Open [os_open_panic]

- **WHEN** a native Open worker panics while staging a document and the caller awaits the request
- **THEN** the request completes once with `OP_ERR_PANIC`, the loading tab is removed, the previous document stays active, the busy cursor is restored if this was the last request, and a later native Open succeeds

#### Scenario: Cancellation wins over an unpublished native Open result [os_open_cancel]

- **WHEN** the user cancels a native Open whose guarded worker has yet to deliver its result
- **THEN** the request completes once with `OP_ERR_CANCELLED`, no failure is shown, and the late worker result leaves the tabs and current document as they were after cancellation

#### Scenario: Memory refusal stays with its native Open request [os_open_memory_concurrent]

- **WHEN** a native Open refused by the memory budget through a ControlServer open route overlaps an independent application or GUI open that fails or is cancelled; each control completion adapter is exercised separately
- **THEN** each completion and protocol response belongs to its own request, the memory refusal retains its existing protocol code, and cancellation takes precedence over a late failure


### Requirement: Native Save reports an error and its publication outcome

Native PSD/PSB Save, Save As and Save a Copy SHALL carry the operation status separately from the file publication outcome. A codec refusal SHALL use `OP_ERR_CODEC`, a file I/O failure SHALL use `OP_ERR_IO`, and a guarded unwind SHALL use `OP_ERR_PANIC`. A panic before rename SHALL have the same preservation and temporary-file cleanup guarantees as any other pre-rename failure. A panic after rename and before durable acknowledgement SHALL be treated as uncertain publication. For Save and Save As, the existing uncertainty notice and recovery rules SHALL apply only to the still-current originating session. Save a Copy SHALL follow the copy-specific uncertainty rule in `interop/save-a-copy`; a replacement session SHALL NOT inherit the original save's protection obligations.

#### Scenario: Native Save distinguishes a codec refusal from an I/O failure [os_save_error_kinds]

- **WHEN** one native save fails in the encoder and another fails to write to the destination
- **THEN** Qt receives `OP_ERR_CODEC` for the encoder refusal and `OP_ERR_IO` for the file error, and both saves report pre-rename failure without changing the original file

#### Scenario: A native Save panics before rename [os_save_panic_before_rename]

- **WHEN** a native Save As worker panics while writing its temporary file
- **THEN** Qt receives `OP_ERR_PANIC` with pre-rename failure, the temp file is gone, and the destination bytes, live path, dirty state, and recovery data are unchanged

#### Scenario: A native Save panics after rename [os_save_panic_after_rename]

- **WHEN** a native save of the current document panics after replacing the file and before durable acknowledgement
- **THEN** Qt receives `OP_ERR_PANIC` with uncertain publication, the destination contains the written snapshot, the live document keeps its path and requires unsaved-changes protection, and a later durable save of the current state clears that obligation

#### Scenario: An uncertain save completes after its session is replaced [os_save_replaced_panic]

- **WHEN** a native save panics after rename and before durable acknowledgement, and its originating session is replaced before completion
- **THEN** the originating attempt reports `OP_ERR_PANIC` and uncertain publication while the replacement session keeps its path, dirty state, history and recovery data unchanged
