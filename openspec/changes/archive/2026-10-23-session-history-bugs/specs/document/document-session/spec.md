# Spec Delta — document/document-session

## MODIFIED Requirements

### Requirement: Document replacement cancels first and installs coherent incoming state

When Revert, recovery restore, Open, New, Open Image, Open As Smart Object, or
Duplicate installs a document, the app SHALL cancel the live session before
installing it, so cancellation can only affect the outgoing document. It SHALL
clear the outgoing selection, the prior-selection memory used by Reselect, the
mask edit target, the active path selection, any path gesture, pending lasso
points and edge map, filter and mode previews, the ruler, pending
opacity/fill/mask-refine previews, the canvas proxy, and the stored PDF options,
except that Revert SHALL preserve the file identity: the document path, the PDF
password and options, and the source format. It SHALL bump the document
generation so document-derived caches cannot serve the incoming document. Open,
New, Open Image, Open As Smart Object, recovery restore, and Duplicate SHALL
discard the outgoing history; Revert SHALL preserve it.

Revert SHALL preserve the outgoing history stack and its named snapshots, the
document path, the PDF password and options, and the source format. On failure
it SHALL leave the document, selection, dirty state, path, history, and any live
session unchanged.

The incoming PDF password SHALL be installed when the incoming document is a
protected PDF opened. Revert SHALL keep and reuse the stored password, so a
later Revert can reopen the file; New, Duplicate, and a non-PDF incoming
document SHALL clear the password. The installed document's history SHALL be the
incoming `Open` state for Open and Open Image, the `Open As Smart Object` state
for Open As Smart Object, exactly one `New` state for New, exactly one
`Duplicate` state for Duplicate, and, for Revert, the preserved outgoing history
plus exactly one appended `Revert` state as the current state. An in-flight open
or filter result belonging to an outgoing document SHALL be discarded by request
or generation identity. Revert SHALL emit one full-change notification for the
installed document; Open, New, and Duplicate SHALL keep their existing signal
policy.

#### Scenario: A live session cannot leak into the new document [ds_replace_cancels]

- **WHEN** a paint stroke, mask stroke, healing gesture, lifting transform, filter preview, or mode preview is live and Revert, New, or Open installs a document
- **THEN** the installed document contains none of the outgoing session's pixels, no edit session is live, and the outgoing handler state cannot commit into it

#### Scenario: Outgoing pointers, previews, and caches are cleared [ds_replace_clears]

- **WHEN** a document with a mask edit target, a selected saved path, a ruler measurement, pending opacity/fill/mask-refine previews, a canvas proxy, and PDF options is replaced by Revert or New
- **THEN** the mask edit target and active saved path are unset, the ruler and pending previews are gone, the document generation changed, and the canvas proxy cannot serve the incoming document; New clears the stored PDF options while Revert keeps the path, PDF password and options, and source format

#### Scenario: PDF password follows the incoming document [ds_pdf_password]

- **WHEN** New or Duplicate follows a protected PDF, or a protected PDF is opened or reverted
- **THEN** New and Duplicate clear the stored password, the PDF open installs the incoming password, and Revert keeps and reuses the stored password so a later Revert can reopen the file

#### Scenario: Reselect is unavailable after a same-size replacement [ds_reselect_after_replace]

- **WHEN** a selection is made and deselected, then a replacement document of the same dimensions is installed, and Reselect is queried or applied
- **THEN** the query reports no prior selection, applying Reselect changes nothing, and the installed document has no selection

#### Scenario: Replacement installs the incoming history [ds_replace_history]

- **WHEN** Open, Open Image, Open As Smart Object, New, or Duplicate installs a document
- **THEN** the history contains exactly the incoming `Open` or `Open As Smart Object` state, one `New` state, or one `Duplicate` state respectively, and that single initial state is the only undo target

#### Scenario: A stale async install is discarded [ds_stale_async]

- **WHEN** an open or filter installation carries a generation or request id that no longer matches the current document
- **THEN** the late result is dropped, the installed document and history are unchanged, and no session state is created

#### Scenario: Replacement keeps the entry point's signal policy [ds_replace_signals]

- **WHEN** Open, New, Duplicate, or Revert installs a document with no live session
- **THEN** Open, New, and Duplicate emit the same full-change or blit signal they emit today and add no second history capture, while Revert emits one full-change notification for the installed document and its `Revert` state is the only capture

#### Scenario: Revert keeps the outgoing history [ds_revert_history]

- **WHEN** Revert reloads an intact file into a document with prior states and named snapshots
- **THEN** the prior states and named snapshots remain and exactly one `Revert` state is appended as the current state; undo then restores the pre-Revert document

#### Scenario: A failed Revert changes nothing [ds_revert_failure]

- **WHEN** Revert cannot read or decode the file, including a wrong PDF password
- **THEN** the document, pixels, selection, dirty state, path, history, and any live session are unchanged
