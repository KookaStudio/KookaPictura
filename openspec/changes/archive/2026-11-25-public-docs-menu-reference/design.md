## Context

`public_docs/` is the user guide published by the Pages workflow. Its pages are
task-oriented; the command catalogue in
`crates/pictura-script/src/command_catalogue/entries.rs` and the running app's
menu registry are the source of truth for what a menu contains. The guard runs
in the guards workflow, which does not build the Qt app, so it reads the
declarative catalogue rather than the running registry.

## Decisions

### One page per top-level menu

Each page follows the menu's own order and groups commands by their submenu.
The visible table gives the command label, its shortcut and a short note; the
group's paragraph explains what the family does and when to use it. Rows link
to the existing task page where one exists.

### Hidden command manifest per page

Each page ends with an HTML comment:

```
<!-- pictura-commands
file.new
file.open
...
-->
```

The manifest is invisible in the rendered guide and makes coverage exact. The
checker maps each catalogue id with a path to its top-level page (or to
`preferences.md` for `edit.preferences.*`) and requires it to appear in that
manifest. A command with no path is panel- or tool-internal and must be listed
in `scripts/public-docs-command-allowlist.txt` with a reason. The checker also
rejects a manifest id that is not in the catalogue and an id filed under the
wrong page.

### Screenshots from the app

The View extra screenshots are captured from a headless Kooka Pictura through
the control server, using a public-domain-style photo already in the guide.
Each extra is switched on alone (`View ▸ Show ▸ None` equivalent), given
content where it needs it (a selection, a path, a note, a slice, count points,
a guide, a moved layer), and grabbed from the canvas. The images are checked in
under `public_docs/images/view-show-*.png`. The capture uses the existing
control protocol only; no app code changes.

### Disabled and Extended-only commands

The running app's `list_commands` reports what is implemented. Disabled menu
commands are listed under **Not yet available** on their page rather than
omitted. Extended-only commands (Count, Measurement Scale, Statistics, 3D, and
the smart-object stack modes) carry an **Extended** marker. The guard only
requires the id to be present, so a disabled command stays covered even if its
status changes later.

## Risks

- The catalogue can drift from the running app. The existing
  `tst_command_tree` keeps the registry and catalogue in step; the guard adds
  documentation coverage on top.
- The screenshots depend on the app's rendering. They are documentation
  captures, not golden images; a future UI change may require recapturing them.
