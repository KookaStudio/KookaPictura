# interop/print Specification

## Purpose
The Print dialog: page setup, scaling, and the raster print output.

## Requirements

### Requirement: Print dialog

File > Print… (Ctrl+P) SHALL open a dialog that previews the flattened image
centred and fitted on the destination's page in the chosen orientation, offers
every available printer and Save as PDF, and prints the image scaled to fit the
printable area with the chosen copies and orientation. Done and Print SHALL keep
the settings as the defaults for the next Print and for Print One Copy; Cancel
SHALL not.

#### Scenario: Preview, orientation, and a PDF job

- **WHEN** the `tst_print` test opens the dialog on a red image, toggles landscape and Show Paper White, cancels, then sets two copies and landscape and prints to Save as PDF
- **THEN** Save as PDF is listed last, the page reads 8.5 in x 11 in portrait with the image centred and 11 in x 8.5 in landscape, the paper tints, Cancel keeps the old defaults, and Print writes a PDF and keeps two copies and landscape as the defaults; a cancelled file choice prints nothing

### Requirement: Print One Copy

File > Print One Copy SHALL print one copy with the last settings without a
dialog when the last destination is an available printer, and SHALL open the
Print dialog otherwise. Both commands SHALL be enabled only with a document.

#### Scenario: The commands follow the document

- **WHEN** the `tst_print` test checks File > Print… and Print One Copy before and after creating a document
- **THEN** Print… carries Ctrl+P, both are disabled without a document and enabled with one, and the printable image is the flattened document

### Requirement: Print Settings page setup reaches the preview and the job

Print Settings SHALL retain the chosen page size and paper source together with
the printer, copies, and orientation. The dialog preview and the page-size label
SHALL use the retained page size. Print and Print One Copy SHALL apply the
retained page size and paper source to the `QPrinter` used for the job,
including Save as PDF. The Print Settings control SHALL be available for the
Save as PDF destination so the page size can be chosen without a printer. Done
and Print SHALL persist the page setup in the last print settings; Cancel SHALL
not. The page-setup dialog SHALL be replaceable by a test provider so the choice
can be exercised without a printer.

#### Scenario: A4 chosen in Print Settings reaches the preview and the job

- **WHEN** a test page-setup provider selects A4 and the Cassette source and the dialog is accepted
- **THEN** the page-size label and preview use A4, the settings retain A4 and Cassette for the next Print, `configurePrinter` with the dialog's settings produces a `QPrinter` whose page size is A4, and a real CUPS queue confirms the paper source on the job (manual check)
