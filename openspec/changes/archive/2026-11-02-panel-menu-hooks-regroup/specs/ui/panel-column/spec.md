# Spec Delta

## ADDED Requirements

### Requirement: Panel menu hooks follow their panel across a regroup

The frame SHALL keep each panel's dynamic menu hook in a frame-owned registry
keyed by the panel's stable object name. A `PanelGroup` SHALL resolve the hook
for its current panel from that registry when the panel menu opens, so the hook
SHALL follow the panel when the group that hosts it is replaced. This SHALL hold
for the fresh-session regroup into Essentials, a workspace apply, and a panel
moved between groups. A hook registered directly on a group SHALL take
precedence over the frame registry. A panel with no hook SHALL keep its static
menu, and a group with no owning frame SHALL keep its existing fallback.

#### Scenario: A fresh session keeps the panel hooks [pmh_fresh]

- **WHEN** a fresh session regroups the staged default groups into Essentials
- **THEN** the Character, Paragraph, and Actions per-widget menus reflect their
  panels' current checked and enabled state, not the static table's defaults

#### Scenario: A workspace switch keeps the panel hooks [pmh_workspace]

- **WHEN** a workspace apply rebuilds the panel groups
- **THEN** every panel's per-widget menu still reflects that panel's state

#### Scenario: A panel moved between groups keeps its hook [pmh_move]

- **WHEN** a panel with a dynamic menu hook is moved from one group to another
- **THEN** the destination group's per-widget menu for that panel reflects the
  panel's state
