# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dbg_menu_entry` | Launch with no document; open Help ▸ Developer | `Pictura Metrics` is enabled, unchecked, no panel | `tst_debug_toolset::menuEntryIsCheckableAndOffAtStartup` |
| `dbg_not_persisted` | Enable, drag the panel, restart | Toolset off, no setting written | `tst_debug_toolset::stateIsNotPersisted` |
