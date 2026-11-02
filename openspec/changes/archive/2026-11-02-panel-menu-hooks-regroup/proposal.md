# Proposal

## Why

`buildPanels` installs the Actions, Character, and Paragraph per-widget menu
hooks on the `PanelGroup`s it creates. `regroupAndApply` harvests the panels,
deletes those groups, and builds fresh ones without the hooks. A fresh session
regroups into Essentials during construction, so the hooks are gone from the
first launch on, and every workspace switch drops them again. Only the Actions
panel keeps working, because `updateHeaderMenu` has a hard-coded Actions
fallback; the Character and Paragraph menus fall back to the static table's
checked and enabled states (#1057).

## What Changes

- **The frame owns the hooks.** `PicturaMainWindow` keeps each panel's menu hook
  in a registry keyed by the panel's stable object name. `buildPanels` registers
  the Actions, Character, and Paragraph hooks there instead of on a group.
- **A group resolves the hook when its menu opens.** `PanelGroup` looks up the
  hook for its current panel from the owning frame's registry (a group-level
  hook still wins), so the hook follows the panel when the group that hosts it
  is replaced by a fresh-session regroup, a workspace apply, or a panel move.
- **The Actions fallback stays** for standalone groups that have no frame, and
  the static menu table still supplies the rows.
- **No visual or menu-content change.** The same menus, rows, and states are
  produced; they are now produced after a regroup as well.

Not in this change: adding new dynamic menu rows or moving any other panel's
menu state into the registry.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/panel-column`: the per-widget panel header menu gains a requirement that a
  panel's dynamic menu hook follows the panel across a regroup, a workspace
  apply, or a move between groups. The existing per-widget menu requirement and
  its static rows are unchanged.

## Impact

- `crates/pictura-app/cpp/frame.h` and `cpp/frame_build.cpp` (the frame-owned
  registry and hook registration).
- `crates/pictura-app/cpp/panels/panel_group.{h,cpp}` and
  `panels/panel_group_menu.cpp` (owning-frame lookup and resolution).
- `crates/pictura-app/cpp/tests/tst_workspaces.cpp` (regression tests).
- `docs/dev/STATE.md` only; no behavioural contract in `docs/` changes.
- No new dependencies, no file-format or session-schema change.
