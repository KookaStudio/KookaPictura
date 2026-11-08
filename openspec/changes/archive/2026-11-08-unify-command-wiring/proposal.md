# Proposal

## Why

Commands are declared once in the Rust catalogue but wired through three
dispatch entries — the menu bar, panel-local tables, and the control socket —
and the panel menus still carry their own id, label, and enablement tables. The
same action therefore behaves differently depending on where it is invoked, and
no panel action can take a shortcut. The control socket also keeps a
hand-maintained modal denylist in C++. One registry should own a command's id,
label, enabled/checked state, shortcut, and modal status for every surface.

## What Changes

- Register menu handlers against the generated `command_ids::` constants.
  Freeze the static leaves that still derive their id at runtime: Color Range,
  Refine Edge, Warp Text, Convert To Paragraph/Point Text, Replace All Missing
  Fonts, the 19 layer-style leaves, Apply Image, Image Size, Canvas Size, Print,
  Print One Copy, Filter Gallery, Read Watermark, the Align/Distribute leaves,
  Auto-Blend Layers, Lens Correction, the Type leaves, and the
  `frame_document_menus.cpp` Image Size, Canvas Size, and Print call sites.
  Keep `commandIdForPath` only for Rust data-catalogue leaves (the adjustment
  catalogue and filter descriptors), and make it reject a derived id the
  catalogue does not own.
- Add the modal-command list to the Rust catalogue and refuse `dispatch_command`
  from that metadata; delete the hand-kept C++ denylist. The existing
  request-scoped dialog guard stays as the second line.
- Add panel-only commands (a catalogue row with no menu path) for panel actions
  that are not menu-bar commands, and group them in the Keyboard Shortcuts
  editor.
- Build the Layers row, mask-thumbnail, and vector-mask menus from registry
  command ids: one row-to-id resolver, registry enablement and labels, dispatch
  by id. Add Layer Mask and Add Vector Mask become CS6 submenus.
- Right-clicking a row makes it the active anchor while preserving the
  selection, so registry handlers (which read the Rust layer target) act on the
  clicked row.
- Route the panel flyout, History, Actions, Character, Paragraph, Adjustments,
  and Properties footer actions through the registry; delete the local `RowSpec`
  enable enum, the `perform*Action` switches, and `runPanelMenuAction`.
- Move the dynamic labels (Duplicate, Delete, Merge) into registry label
  providers, so the menu bar, row menu, and flyout show one label.
- Add parity tests that compare each command across the surfaces that offer it,
  and assert that panel actions are listed in the shortcuts editor.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/command-registry`: handlers register against frozen ids; the
  catalogue carries the modal id list and panel-only commands; label providers
  are the source for the menu bar, row menu, and flyout; every surface
  dispatches by id.
- `ui/layers-panel`: the row menu is registry-built, a right-click makes the
  clicked row the anchor, and Add Layer Mask / Add Vector Mask are submenus
  whose entries agree with the menu bar.
- `ui/layers-mask-commands`: row-menu mask enablement and Apply availability
  come from the registry, and the mask rows are submenus.
- `ui/keyboard-shortcuts-editor`: panel commands are listed, grouped by panel.
- `ui/layers-delete-guard`: the Properties footer Delete is one of the gated
  entry points.

## Impact

- `pictura-script`: the command catalogue gains a modal id list and panel-only
  rows; the generated constant list covers the static leaves that currently
  derive their id at runtime (the full freeze set above).
- `pictura-app` Rust bridge: the catalogue exposes the modal id list; the
  path-to-id bridge validates against the catalogue.
- `pictura-app` C++: `commands.{h,cpp}`, the regenerated `command_ids.h`,
  `control_server.cpp`, `frame_menus*.cpp`, `frame_build.cpp`,
  `panels/layers_panel_menu.cpp`, `panels/panel_group_menu.cpp`, and the
  panel files for History, Actions, Character, Paragraph, Adjustments, and
  Properties.
- Qt tests: `tst_command_tree`, `tst_layers_panel*`, `tst_layers_row_menu`,
  `tst_layers_mask_commands`, `tst_smart_object_actions`, `tst_actions_panel*`,
  `tst_type_session_panels`, `tst_adjustment_presets`, `tst_last_layer`,
  `tst_merge_commands`, `tst_properties_panel`, `tst_control_modal_guard`, and
  a new parity suite; `CMakeLists.txt` and the test list gain the new files.
- No new dependencies. This change is for #277 and closes #830.
