# Spec Delta

## ADDED Requirements

### Requirement: App dialogs do not dim the parent window

The shell MUST present every application dialog without a modal window hint that makes the window compositor dim or fade the dialog's parent window (for example KWin's "Dialog Parent" effect). While a dialog is open the shell SHALL block input addressed to the dialog's parent top-level window with an event filter and run the dialog's own event loop, instead of calling `QDialog::exec()` on a window-modal dialog. A dialog with no parent (a test or headless context) MAY use `exec()` directly, and a dialog that must hand off to a platform/portal chooser MAY keep the platform's own modality.

#### Scenario: A dialog opens non-modally over the main window

- **WHEN** an application dialog is opened over the main window
- **THEN** it is shown with `Qt::NonModal` and the main window is not dimmed by the compositor

#### Scenario: The parent stays input-blocked while the dialog is open

- **WHEN** a dialog is open and the user clicks or types at the parent top-level window
- **THEN** that input is swallowed and the dialog and its popups remain interactive

#### Scenario: Closing returns the dialog's result

- **WHEN** the dialog is accepted or rejected
- **THEN** the presenting code receives the same result code a modal `exec()` would have returned
