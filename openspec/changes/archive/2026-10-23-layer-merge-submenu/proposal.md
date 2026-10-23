# Change: Group the Layer merge commands under a Merge submenu

## Why

At larger UI scales the Layer menu is taller than the available screen. Qt's
native multi-column fallback fills the first column to the full screen height and
spills only the last one or two rows into a second column, so the menu stays
awkwardly tall and its lower commands sit at the screen edge. The merge commands
are a cohesive trailing group, so folding them into one submenu shortens the
top-level Layer list enough to avoid that fallback.

## What Changes

- Move Merge Layers, Merge Visible, Stamp Visible, Stamp Selected, Flatten
  Image, and Merge Clipping Mask from the Layer top level into a new
  `Layer > Merge` submenu.
- Keep the six commands' identifiers, labels, default shortcuts, handlers, and
  enablement unchanged; only their menu path changes.
- Leave `Layer > Matting` as its own top-level submenu.

## Impact

- Affected specs: `document/command-registry`
- Affected code: `crates/pictura-app/cpp/command_tree.cpp`,
  `crates/pictura-app/cpp/tests/tst_command_tree.cpp`,
  `crates/pictura-app/cpp/tests/tst_merge_commands.cpp`
