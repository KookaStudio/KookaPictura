# Proposal

## Why

Issue #504 tracks the File-menu stubs found by the #486 docs/code parity audit,
but the audit's evidence (`command_tree.cpp`) has since been replaced by the
Rust command catalogue, and most of the File tree now ships wired commands.
Re-auditing the current tree leaves exactly two behaviours that no other issue
owns: a Linux replacement for `File > Browse in Bridge…`, and the
`File > Scripts > Statistics…` script. The remaining disabled leaves are owned
by #558, #617, #619, #620, and #630; `Check In…` and `Import ▸ WIA Support`
are platform-only rows that stay disabled. #504 can close honestly only by
shipping the unowned pair and re-pointing the register at the rest.

## What Changes

- Add `File > Browse…` (`file.browse`, `Ctrl+Alt+O`) immediately after
  `Open`, in the CS6 position of Browse in Bridge. Adobe Bridge is a declared
  Linux non-goal; the command opens the active document's folder (home when
  there is none) in the OS file manager instead.
- Implement `File > Scripts > Statistics…` (`file.scripts.statistics`,
  Extended only). The **Image Statistics** dialog collects sources with
  `Use: Files / Folder`, a file list, `Browse…`, `Remove`, and
  `Add Open Files`, a stack-mode menu, and
  `Attempt to Automatically Align Source Images`. Accepting loads the sources
  through the shipped Load Files into Stack pipeline as one embedded Smart
  Object and applies the chosen stack mode. The last accepted mode and align
  choice persist in the session.
- Add `File > Scripts > Browse…` (`file.scripts.browse`) as a disabled
  catalogue row. Running a script needs the scripting host (#620/#621), which
  does not exist and whose ExtendScript/JSX compatibility is a declared
  non-goal; this change adds the menu row only, owned by #620.
- Reconcile the File-menu record: correct the documented File order (Browse in
  Bridge precedes Open As) and the `Statistics…` label, move the parity
  register's UI-8 to `PARTIAL` with the shipped pair and per-leaf owners, and
  fold `Crop And Straighten Photos` into #619.

**Non-goals:** Adobe Bridge and Mini Bridge; any JavaScript/JSX runner, script
host, DOM, notifiers, console, or script security prompt; the remaining
Automate/Scripts engines (Image Processor, Conditional Mode Change, PDF
Presentation, Contact Sheet II, Picture Package, Photomerge, Merge To HDR Pro,
Create Droplet, Render Video, Script Events Manager); removing the
platform-only `Check In…` and `Import ▸ WIA Support` rows; `Close and Go to
Bridge…` (omitted with the Bridge non-goal).

## Capabilities

### New Capabilities

- `interop/file-browse`: the `File > Browse…` command, its menu position and
  shortcut, its folder/home target, and its independence from Adobe Bridge.
- `interop/image-stack-statistics`: the Statistics command, its Extended-only
  availability, the merge pipeline (stack, Smart Object, stack mode), align
  forwarding, the two-source minimum, failure handling, and the persisted
  mode/align choice.
- `ui/statistics-dialog`: the **Image Statistics** dialog's controls,
  defaults, enablement, source list behaviour, folder Browse, Add Open Files
  warning, and remembered choices.

### Modified Capabilities

None. The disabled `File > Scripts > Browse…` row is already covered by the
existing `document/command-registry` requirement that commands without a
handler appear disabled; no requirement text changes.

## Impact

- Command catalogue: `crates/pictura-script/src/command_catalogue/entries.rs`,
  `const_names.rs`, `command_catalogue.rs` (Extended edition list and its
  test), the two golden tables, and the generated
  `crates/pictura-app/cpp/command_ids.h`.
- File-menu wiring: `crates/pictura-app/cpp/frame_commands.cpp`,
  `frame.h`/`frame.cpp` for the URL-opener helper, and `dialogs.h`/`dialogs.cpp`
  for the shared stack source suffixes and filters.
- Statistics: new `crates/pictura-app/cpp/statistics_dialog.{h,cpp}`,
  `session.h`/`session.cpp` for the remembered options, and
  `cmake/sources/documents.cmake`.
- Tests: new `tst_file_menu` and `tst_statistics`, an Extended/Standard case in
  `tst_edition_gating`, and the test CMake list.
- Documentation: parity register UI-8, `docs/02-ui-ux/menus.md`,
  `ROADMAP.md`, and `docs/dev/STATE.md`.
- No new dependency: the merge reuses `load_files_into_stack` and
  `set_smart_object_stack_mode`; the dialog is Qt only.
