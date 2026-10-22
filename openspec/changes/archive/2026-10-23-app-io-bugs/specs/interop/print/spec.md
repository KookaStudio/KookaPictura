# Spec Delta: interop/print

## ADDED Requirements

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
