# Whole-Document Operation Status

## ADDED Requirements

### Requirement: Whole-document commands publish edits only on success

An operation using the shared whole-document command path SHALL deliver its operation status whether executed immediately or on a worker. An unwinding panic during staged engine work SHALL complete as `OP_ERR_PANIC`. An invalid-argument preflight refusal, including one before the shared helper, SHALL report `OP_ERR_INVALID_ARG` through the Qt command adapter, while admission refusals and legitimate no-ops retain distinct outcomes. A cancellation observed before publication SHALL report `OP_ERR_CANCELLED`. None of these outcomes SHALL publish staged pixels, geometry, or command results, advance the history index, add a history state, change the document's dirty state, or leave the command busy. A worker whose source generation, revision or history identity has changed SHALL complete its own attempt as `OP_ERR_CANCELLED` without failure feedback, discard its staged result and leave newer state and status intact. A successful command SHALL publish once and report `OP_OK` with its command result.

#### Scenario: A whole-document engine panics after changing staged pixels [os_document_panic]

- **WHEN** a rotation panics after modifying staged state, once on the synchronous path and once on a worker
- **THEN** each command completes with `OP_ERR_PANIC`, the visible document and history stay intact, and a later successful rotation reports `OP_OK` with one new history state

#### Scenario: A whole-document command refuses invalid input [os_document_refused]

- **WHEN** a shared whole-document command returns a validation refusal after a prior successful command
- **THEN** Qt receives `OP_ERR_INVALID_ARG` without a previous command's result, and the document, dirty state, and history remain as they were after the prior command

#### Scenario: A whole-document cancellation has a distinct outcome [os_document_cancel]

- **WHEN** a whole-document worker is cancelled before its result is installed
- **THEN** Qt receives `OP_ERR_CANCELLED` without error feedback or a committed result, and the pixels, dirty state and history are unchanged

#### Scenario: A stale ordinary worker does not publish [os_document_stale]

- **WHEN** an ordinary shared whole-document worker is held while a newer edit or replacement changes its source identity
- **THEN** the original attempt completes as cancelled without failure feedback, its staged result is discarded, and the newer document, history and status remain intact
