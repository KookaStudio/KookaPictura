# Proposal: history-popup-undo-routing

## Why

Ctrl-Z currently stops reaching the application's Edit > Undo command while the History panel is hosted in its iconic-mode popup, leaving document edits unreachable by keyboard until the popup closes (#657). The fix should preserve the existing undo command and make its accelerator available from that popup.

## What Changes

- Keep the Edit > Undo accelerator available while the History panel's iconic flyout is active.
- Preserve native Ctrl-Z undo in focused editable text controls.
- Add focused Qt Test coverage for both behaviors.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/history-panel`: specify that Ctrl-Z remains available while the History panel is hosted in its iconic flyout, while text controls retain their own undo.

## Impact

- `crates/pictura-app/cpp/frame_menus_edit.cpp` — shortcut routing for Edit > Undo.
- `crates/pictura-app/cpp/tests/tst_edit_shortcuts.cpp` — regression coverage.
- No API or dependency changes; no `docs/` changes.
