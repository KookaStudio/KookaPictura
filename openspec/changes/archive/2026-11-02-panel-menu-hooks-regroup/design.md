# Design

## Why the hook belongs to the frame

A panel menu hook is a property of the panel, not of the group that currently
hosts it: it updates that panel's own rows and enablement when its menu opens.
The group is a transient container — `regroupAndApply` deletes and rebuilds
every group on a fresh-session regroup or a workspace apply, and a drag can move
a panel to a different group. Storing the hook on the group therefore ties the
panel's menu state to an object that may disappear or may never have received
the hook.

The frame already owns the panels and outlives every group, so it is the stable
owner. The registry is a `QHash<QString, std::function<void(QMenu*)>>` keyed by
the panel's `objectName`, the same key the rest of the frame's panel lookups use.

## Resolution, not re-application

The issue's suggested shape is "keyed by panel name on the frame and re-applied
when a panel joins a group". Re-applying requires catching every join point:
`buildPanels`, `regroupAndApply` (both the normal build and the overflow group),
the drag/drop destinations, the float host, and any future group constructor.
Missing one reintroduces the same class of bug.

Resolving the hook when the menu opens removes that set of join points entirely.
`updateHeaderMenu` already knows the current panel name and already installs an
`aboutToShow` handler; the handler now asks the owning frame for the hook at
show time. Whichever group hosts the panel at that moment gets the right hook,
and no code has to move hooks between containers.

Order of resolution:

1. a hook registered directly on the group (kept for standalone groups and
   tests; `PanelGroup::setPanelMenuHook` is unchanged),
2. the owning frame's registry,
3. the existing Actions widget fallback, then the static table.

## Finding the frame from a group

`PanelColumn::owningFrame()` already solves the same problem: `window()` is the
frame for a docked column, but the floating overlay for a floated one, whose
parent is the frame. `PanelGroup` uses the same two-step lookup (`window()`,
then `window()->parentWidget()`). A group inside a design-system showcase dialog
also finds the frame, but no showcase panel shares a real panel's object name,
so the lookup is inert there.

## Lifetime

The frame constructs every group and owns the panels; groups are deleted before
or with the frame, and the hook map is a frame member. The `std::function`
copies returned by the accessor are self-contained lambdas capturing the frame
and a panel pointer, both alive for the duration of the menu show.

## Alternatives considered

- **Re-apply the frame map at every join point.** Correct but fragile: it needs
  a call in each group-building and panel-moving path, and a new path can forget
  it. Rejected.
- **Special-case Character and Paragraph in `updateHeaderMenu` like Actions.**
  The Actions special case was already noted as not scaling when the hook was
  introduced; adding two more concrete casts makes the group know every panel
  that has a dynamic menu. Rejected.
- **Transfer the group's hook map during `harvest`.** Only fixes the regroup
  path, not a drag between groups, and keeps the hook tied to a dying group
  until it is copied.
- **Store the hook on the panel widget.** Would avoid the frame lookup, but the
  hook needs frame state and would have to be registered through a new panel
  API; the frame registry is smaller.

## Edge cases

- **No frame** (a standalone `PanelGroup` in a test or the design-system
  showcase): the frame lookup returns null and the group-level hook or the
  Actions fallback applies, exactly as before.
- **No hook for the panel**: the static menu table is used unchanged.
- **Panel moved while its menu is open**: the menu is rebuilt on the next
  `updateHeaderMenu`, and the `aboutToShow` handler captures the panel name at
  build time, so it cannot read the wrong panel's hook.
- **Frame teardown**: groups are destroyed before the frame's members; a menu is
  not shown during teardown.
