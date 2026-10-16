# Proposal: print-dialog

## Why

Issue #66: import the Print dialog from photorust
(`shell/src/dialogs/PrintDialog.cpp`, `MainWindow::printDocument` /
`printOneCopy`). File > Print… (Ctrl+P) and Print One Copy were inert
placeholders.

## What Changes

- `print_dialog.*` (new): CS6's Print dialog — a page preview (paper size from
  the destination, portrait / landscape, Show Paper White), Printer Setup
  (every CUPS queue plus **Save as PDF**, copies, Print Settings… for the
  Qt / CUPS print dialog, orientation), Color Management (Printer Manages
  Colors; the document profile is sRGB because the flattened image is), and
  Cancel / Done / Print. The image prints centred and scaled to fit the
  printable area (CS6's Scale To Fit Media).
- Done and Print keep the settings as the defaults (`lastPrintSettings`), as
  CS6 does; Cancel does not.
- File > Print… opens it; File > Print One Copy prints one copy to the last
  printer without a dialog, or opens the dialog when no queue is remembered
  (Save as PDF needs a file name).
- Save as PDF is a Linux addition (`WF-013`'s "Linux reality": with no valid
  printer `QPrinter` writes a PDF); photorust's Print One Copy always wrote a
  PDF.
- Build: links **Qt6::PrintSupport** (part of the system Qt and of CI's
  `install-qt-action` base install) for `QPrinter` / `QPrintDialog` /
  `QPrinterInfo`.
- Tests: Qt Test `tst_print` (prints a real PDF).

## Capabilities

### New Capabilities

- `interop/print`: File > Print and Print One Copy.

## Impact

- `pictura-app` (C++), CMake. New Qt module dependency: Qt6::PrintSupport
  (no new crate).

## Provenance

Layout and fit-to-page printing from photorust's PrintDialog; behaviour from
`WF-013` (`docs/10-workflow-io/printing.md`). Ceiling (`ponytail:`):
printer-managed colour only (Photoshop Manages Colors, Separations, Match Print
Colors, Gamut Warning shown disabled); no Position and Size fields, Print
Selected Area, output marks, Description, or 16-bit output; settings last for
the session.
