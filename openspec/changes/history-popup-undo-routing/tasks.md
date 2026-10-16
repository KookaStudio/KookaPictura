# Tasks

## 1. Regression tests

- [x] 1.1 Add a Qt Test that opens the History icon flyout, presses Ctrl-Z, and verifies one document state is undone without requiring the flyout to close.
- [x] 1.2 Add a Qt Test that verifies Ctrl-Z in a focused editable text control undoes the text edit while leaving document history unchanged.

## 2. Shortcut routing

- [ ] 2.1 Set the existing Edit Undo action to application shortcut context while preserving its current handler and binding; verify the popup regression test passes.

## 3. Validation

- [ ] 3.1 Run the focused Qt Test suite and the repository-required validation gates; record results and include the manual desktop check in the PR body.
