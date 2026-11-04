# Spec Delta

## MODIFIED Requirements

### Requirement: The change report drives display and notification

Session and edit operations SHALL report what changed through one presentation
value: no display change, one or more bounded document regions, or a full
rebuild, plus the dirty value to apply, whether the content revision advances,
and whether a full change notification is required. A bounded region SHALL be
reported only after the producer has folded its pixels into the displayed level
0 and pyramid; the report SHALL NOT carry pixel buffers. The presenter SHALL
apply the dirty and content-revision bookkeeping, emit the region blit(s) for
bounded regions, perform a full rebuild for a rebuild request, and emit a
notification only when the flag is set. The report MUST preserve the existing
signal counts: a bounded region does not by itself imply a full change
notification, a bounded region MAY require one (healing, patch, red-eye, and
batch-property commits do), and a metadata, selection, or Work/Saved path
restoration can notify without a pixel change. An empty region list emits no
blit.

#### Scenario: A bounded change blits without a full notification [ds_report_region]

- **WHEN** a paint or mask stroke is cancelled or committed with bounded affected rectangles and no notification flag
- **THEN** the presenter emits the region blit(s) and does not emit a full change notification

#### Scenario: A bounded change can also notify [ds_report_region_notify]

- **WHEN** a healing commit, patch, red-eye, or a layer-opacity/fill batch commit reports bounded regions and the notification flag
- **THEN** the presenter emits the region blit(s) and exactly one full change notification

#### Scenario: A notification-only change emits no blit [ds_report_notify]

- **WHEN** a Work or Saved path gesture is aborted, restoring the path and selection without changing document pixels
- **THEN** the report carries a notification but no display region, and the presenter emits a full change notification without a region blit

#### Scenario: A shape-layer path abort rebuilds [ds_report_shape_path]

- **WHEN** a path gesture that edited a shape layer's path is aborted and the pre-gesture document is restored
- **THEN** the report requests a rebuild or a bounded region and a notification, and the presenter does not treat it as notification-only

#### Scenario: A full change rebuilds and notifies [ds_report_rebuild]

- **WHEN** a mode preview is cancelled and the pre-preview document is restored
- **THEN** the report requests a full rebuild, the presenter rebuilds from the restored composite, and a full change notification is emitted
