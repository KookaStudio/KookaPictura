# document/prepared-plan-job Specification

## Purpose

Defines one lifecycle for a plan computed on a worker from a document snapshot:
who owns the worker, cancellation, progress and captured revision, how a
finished run is joined once, and when its result may apply. The user-visible
cancel and history guarantees stay owned by
`document/long-operation-progress`, `document/auto-align-layers`, and
`document/auto-blend-layers`; this capability owns only the shared seam they
call.

## Requirements

### Requirement: One owner holds a prepared plan run

A prepared-plan run SHALL be owned by one lifecycle object that holds the
worker, the cancellation and progress state, the document content revision
captured when the run started, and the worker's joining handle. A start SHALL
receive the document snapshot and the content revision captured before the
worker starts; the caller supplies both and the owner does not snapshot the
document itself. A start SHALL be refused while the previous run is still held,
whether it is running or finished and not yet landed. Polling SHALL report
whether the worker is still running and the progress the engine wrote, and SHALL
report progress zero when no run is held. The owner SHALL NOT compute or apply a
plan itself.

#### Scenario: One run at a time [ppj_one_run]

- **WHEN** a run is started and a second start is attempted before the first is landed
- **THEN** the second start is refused and the first run is unchanged

#### Scenario: Polling reports the held run [ppj_poll]

- **WHEN** a run is held and polled for running state and progress
- **THEN** polling reports the worker's running state and the progress the engine wrote, and reports progress zero after the run is landed

#### Scenario: A landed run makes room for another [ppj_restart]

- **WHEN** a run is started, landed, and a new run is started
- **THEN** the new start is accepted and owns a fresh worker and run, with the revision captured at the new start

### Requirement: A finished run lands exactly once

Landing a run SHALL take the held run and join its worker once before deciding
the outcome, and SHALL yield at most one plan to the caller. A second landing
attempt with no held run SHALL yield nothing and MUST NOT apply a plan again.
Polling, stopping or discarding a run MUST NOT apply its plan.

#### Scenario: A finished plan lands once [ppj_once]

- **WHEN** a run whose worker finished is landed twice
- **THEN** the first landing yields its plan and the second yields nothing

#### Scenario: Completion before the first poll still lands [ppj_before_poll]

- **WHEN** a worker finishes before the run is polled for the first time and the run is landed
- **THEN** landing joins the finished worker and yields its plan

### Requirement: Only a current, wanted plan may apply

Landing SHALL take the held run, join its worker once, and only then classify the
outcome in this order: a join panic is reported as failed and MUST NOT propagate
to the caller; a requested stop is reported as cancelled even when the worker
returned a plan; a worker that returned no plan is reported as cancelled; a
document content revision at landing that differs from the revision captured at
start is reported as stale; otherwise the plan is yielded as ready. A requested
stop therefore wins over a stale revision, so a cancellation arriving after the
engine's last check cannot commit. A stale, cancelled or failed landing SHALL
yield no plan, so the caller cannot mutate the document or record a history
state for it.

#### Scenario: A stale plan is dropped [ppj_stale]

- **WHEN** the document content revision changed while the worker ran and the finished run is landed
- **THEN** landing reports stale and yields no plan

#### Scenario: A cancelled plan is dropped [ppj_cancel]

- **WHEN** a run is asked to stop and is landed even though its worker returned a plan
- **THEN** landing reports cancelled and yields no plan

#### Scenario: A panicking worker is dropped [ppj_panic]

- **WHEN** the worker panics and the run is landed
- **THEN** landing reports failed, yields no plan, and the panic does not reach the caller

### Requirement: Dropping a run cancels without joining

Dropping the lifecycle owner, or discarding its held run, SHALL ask the worker
to stop and SHALL NOT join it. The worker SHALL be left to observe the stop and
exit on its own. A discarded run SHALL release the held-run slot so a later
start is accepted. The application SHALL discard a held run when the document it
was prepared from is replaced.

#### Scenario: Dropping the owner cancels its worker [ppj_drop]

- **WHEN** the lifecycle owner holding a run is dropped
- **THEN** the worker's cancellation flag is set and the caller did not wait for the worker to finish

#### Scenario: Discarding a run allows a new start [ppj_discard]

- **WHEN** a held run is discarded and a new run is started
- **THEN** the discard asked the previous worker to stop without joining it and the new start is accepted

#### Scenario: Replacing the document discards its run [ppj_replaced]

- **WHEN** a run is held and the document it was prepared from is replaced
- **THEN** the held run is discarded, landing it yields nothing, and a new start is accepted and lands
