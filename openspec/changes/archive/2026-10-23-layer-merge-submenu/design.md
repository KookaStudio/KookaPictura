# Design

The command table stores menu paths, so `command_tree.cpp` changes the six
merge entries from `{"Layer", "<label>"}` to `{"Layer", "Merge", "<label>"}`.
`CommandRegistry::buildMenuBar` creates the submenu from the shared path prefix,
exactly like the other Layer submenus, so the builder needs no change. The
commands still resolve by identifier, so handlers, enablement providers,
keyboard shortcuts, search, and the shortcut editor are untouched.

The separator before the group stays at the Layer top level, so `Merge` becomes
its own separator-delimited group and `Matting` follows it. This removes five
top-level rows (six commands collapse to one submenu row) without changing any
command behaviour.

The native multi-column fallback remains for any menu that is still taller than
the screen; this change only keeps the Layer menu under that threshold on the
screens where it currently overflows.
