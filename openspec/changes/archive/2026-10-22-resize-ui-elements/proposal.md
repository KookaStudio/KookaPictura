# Resize UI elements

## Why

On high-resolution monitors the fixed 12 px UI font and the fixed pixel metrics
around it make the chrome hard to read. Photoshop CS6 exposes a `UI Font Size`
setting (Tiny / Small / Medium / Large) that requires a restart. The owner asked
for a finer, live control: 10 % steps from 75 % to 200 %, reachable from both
the Window menu and Preferences, affecting all application chrome but not the
document image.

## What Changes

- Add a UI scale percentage to the theme: normalized choices 75, 80, 90, 100,
  110 … 200; default 100; `Theme::scaled(px)` for runtime chrome metrics.
- Scale the application font and the stylesheet's pixel metrics, so menus,
  buttons, fields, tabs, and dialog text grow or shrink together.
- Scale the explicit shell chrome that does not come from the stylesheet:
  status bar, tool slots, panel-column widths, panel icon buttons, tab fonts,
  and menu-corner icons.
- Add `Window > UI Size: N%`, `Enlarge UI Size`, `Decrease UI Size`, and
  `Reset UI Size`.
- Add a `UI Size` control to `Edit > Preferences > Interface`.
- Apply a change immediately, with no restart, and persist it in the session.
- Leave document pixels, zoom, and canvas rendering untouched.

## Impact

- Affected capability: `ui/ui-scaling` (new), `ui/bundled-fonts` (modified).
- Code: `theme.*`, `fonts.*`, `frame.*`, `frame_*`, `commands.h`,
  `command_tree.cpp`, `preferences_dialog.*`, `toolbox.*`,
  `panels/panel_column*`, `panels/panel_group.*`, `session.*`.
- Deliberate divergence: `docs/02-ui-ux/application-frame.md` and
  `docs/02-ui-ux/accessibility.md` describe CS6's four-step, restart-only UI
  font size. This change follows the owner's explicit live percentage request;
  the docs are not edited by this change.
