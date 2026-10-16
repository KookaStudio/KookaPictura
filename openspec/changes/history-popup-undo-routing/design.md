# Design: history-popup-undo-routing

## Context

See `proposal.md` for the reported behavior. `CommandRegistry` creates the Edit
Undo menu action with Ctrl-Z, and `wireEditHistory()` attaches the existing
handler that prioritizes tool-session undo before document history. The History
flyout is a separate `Qt::Popup` window, so the main window's default
`Qt::WindowShortcut` is not active while that popup owns focus.

## Goals / Non-Goals

**Goals:**

- Make the existing Edit Undo action active when focus is inside the History
  flyout.
- Keep the existing tool-session and document-history behavior.
- Preserve native Ctrl-Z handling in editable text controls.

**Non-Goals:**

- Add a second undo handler or change undo/redo semantics.
- Re-route unrelated application shortcuts from every popup.

## Decisions

### Use the existing action with application shortcut context

Set the Edit Undo action's shortcut context to `Qt::ApplicationShortcut` when
the main window wires its history commands. This keeps the same customizable
Ctrl-Z binding, enabled state, and handler while making the accelerator active
from the popup top-level window.

Adding another `QShortcut` to the History panel or flyout would duplicate the
binding and could bypass the existing command handler, including its tool
session undo. Forwarding all popup key events to the main window would broaden
the change to unrelated keys.

### Keep text editing precedence

Leave shortcut handling to Qt's normal shortcut resolution so an editable text
control can claim Ctrl-Z for its own undo. Add a regression test that verifies
the text edit changes while the document history remains untouched.

## Risks / Trade-offs

- **The application action is eligible while another app window is active** →
  editable text controls keep their native undo through shortcut override; the
  regression test covers that case. The existing command handler continues to
  prioritize tool-session undo.

## Migration Plan

None. The change affects runtime shortcut routing only and adds no persisted
state or dependency.
