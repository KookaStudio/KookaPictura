## ADDED Requirements

### Requirement: Vanishing Point planes are saved with the document

Saving as PSD, PSB, TIFF or JPEG, and the crash-recovery snapshot, SHALL
write the document's planes; other formats SHALL drop them. Opening or
reverting a document SHALL restore the saved planes when they fit its
canvas, so the dialog reopens on them. Planes drawn on a canvas of another
size SHALL be neither saved nor restored. OK that changes the planes,
including deleting the last one, SHALL mark the document modified without
adding a history state, and it SHALL stay modified until it is saved; OK
with the planes unchanged and Cancel SHALL leave the mark as it was.

#### Scenario: Saved planes reopen [vp_files_reopen]

- **WHEN** `tst_vanishing_point_files::planesReopenFromSavedFiles` draws two
  planes, sets the first's Grid Size to 40, and saves as PSD, PSB, TIFF
  (when Qt has a TIFF writer), JPEG and PNG, reopening each in the dialog
- **THEN** each save leaves the document unmodified; PSD, PSB, TIFF and
  JPEG reopen with both planes at the same corners and Grid Size 40, and
  the PNG reopens with none

#### Scenario: A plane-only OK marks the document modified [vp_files_modified]

- **WHEN** `tst_vanishing_point_files::planeOnlyOkMarksTheDocumentModified`
  cancels a new plane, presses OK with no change, draws a plane and presses
  OK, saves, reopens the dialog and presses OK, then deletes the plane,
  presses OK, saves and reopens the file
- **THEN** only the plane drawn and the plane deleted mark the document
  modified, neither adds a history state, and the reopened file has no
  planes

#### Scenario: Planes must fit the canvas [vp_files_fit]

- **WHEN** `pictura-app::saved_planes_must_fit_the_canvas` reads and writes
  planes for a 100×80 document, an empty set, and a 120×80 document
- **THEN** the 100×80 planes round-trip, the empty set writes no block, and
  the 120×80 document neither reads nor writes them
