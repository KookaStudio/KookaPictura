# Spec Delta

## ADDED Requirements

### Requirement: Destructive adjustment dialog work runs off the GUI thread

A destructive adjustment dialog built on the shared `AdjustmentDialog` SHALL
compute its preview from a copy-on-write clone of the live document on a worker
thread, and the GUI thread SHALL update the live layer only when the result
lands. Applying a result SHALL install the worker's composited changed region,
or refresh only the visible section; the GUI thread SHALL NOT composite the
whole layer. Applying a result SHALL record no history. Parameter changes SHALL
coalesce so one worker runs at a time and only the latest result is applied.
Cancel SHALL drop a pending result and restore the pre-dialog pixels
bit-identically. OK SHALL apply the whole layer on a worker as one state named
for the adjustment. The GUI thread SHALL keep processing events while a preview
or an apply runs. A layer-mask target MAY keep the mask-sized synchronous path.

#### Scenario: The event loop stays live during a preview [ia_dialog_worker]

- **WHEN** an adjustment dialog's preview is held in flight by the test seam
- **THEN** a repeating timer keeps firing with no gap over 50 ms and the canvas keeps repainting

#### Scenario: Only the latest values land [ia_dialog_latest]

- **WHEN** a second parameter change arrives while the first preview is being computed
- **THEN** the canvas shows the result for the second change and the first result is not applied

#### Scenario: Cancel restores the pre-dialog pixels [ia_dialog_cancel]

- **WHEN** an adjustment dialog is previewed and then cancelled
- **THEN** the layer and composite are bit-identical to their pre-dialog state and no history state is recorded

#### Scenario: OK applies off the GUI thread [ia_dialog_apply]

- **WHEN** OK is pressed after a preview
- **THEN** the whole layer is adjusted on a worker, one history state named for the adjustment is recorded, and the GUI thread keeps processing events while the worker runs

#### Scenario: A layer-mask target keeps the synchronous path [ia_dialog_mask]

- **WHEN** a destructive adjustment dialog previews with a layer-mask target
- **THEN** the dialog uses the mask-sized synchronous path and Cancel restores the mask exactly
