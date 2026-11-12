## MODIFIED Requirements

### Requirement: A dialog reached during a request is refused

The control server SHALL cancel any dialog or menu shown while a request runs and
SHALL answer that request `refused`, naming the dialog, so that no request waits
on a person. The progress window of a background job SHALL NOT be cancelled. A
dialog a person opens outside a request SHALL NOT be affected.

#### Scenario: An unlisted dialog does not hang the request [ac_guard_cancels]

- **WHEN** a client dispatches Image Size, which opens a dialog
- **THEN** the dialog is cancelled, the response is `refused`, and the document
  is unchanged

#### Scenario: Every command answers [ac_guard_every_command]

- **WHEN** every implemented and enabled command is dispatched in turn
- **THEN** each one gets a response and no dialog stays open

#### Scenario: A person's dialog is left alone [ac_guard_outside_request]

- **WHEN** a dialog is shown while no request is running
- **THEN** it stays open

#### Scenario: A job's progress window runs to the end [ac_guard_job_progress]

- **WHEN** a client's `layer_op` `free_transform` starts a commit job that runs
  long enough for its cancellable progress window to show
- **THEN** the window is not cancelled, the job lands, the response is `ok`, and
  history gains one "Free Transform" state
