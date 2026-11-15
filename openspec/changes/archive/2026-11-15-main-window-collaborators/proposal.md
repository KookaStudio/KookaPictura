# Proposal

## Why

`PicturaMainWindow` is the shell's only seam: documents, window hosts, panels,
workspaces, command handlers, file open/save, recovery, chrome and test hooks are
all methods of one class. At the issue's measurement it is a 1,200-line header
(now exactly the file-size cap, so a one-line addition already fails
`check-file-size.sh`), with 14,910 lines of `frame_*.cpp`, 159 members, and 221
`frame.h` includers: 25 production, 35 self-test, 159 `tst_*.cpp` suites, and 2
test helper headers (222 `tst_*.cpp` suite files total, 233 sharded CTest
entries). A panel or menu change cannot avoid the frame header, and no
collaborator can be constructed or tested without the whole shell.

Issue #1166 (part of the #1159 review) asks for the frame to become wiring over
owned collaborators. The sibling review items have landed (#1160, #1161, #1162,
#1163, #1164); this is the shell half of the same review.

## What Changes

- Add four owned collaborators and move state ownership with the seam, not just
  method names:
  - **DocumentRegistry** owns open documents, document windows, the active
    window, and the tab/float/tile hosts. It reports changes through signals and
    receives a `DocumentHooks` value from the composition root for canvas/view,
    colour/GPU, recent, refresh, panel-refresh, and guide wiring.
  - **PanelHost** owns panel widgets, panel groups, panel columns, the tools
    column, the central splitter, the workspace store, `panelsHidden_`, and
    panel-column visibility. It owns the panel menu hook store and panel
    binding/refresh.
  - **ImportSaveCoordinator** owns the async open pipeline, save/save-as/revert,
    document close prompting/saving, PDF/EPS/Open As import, recent files,
    save-format options, smart-object edit sessions, and crash recovery.
  - **MenuRegistrar** owns the `CommandRegistry` and all command-handler,
    provider, and context-menu registration, and builds the menu bar. Frame-owned
    chrome commands are registered by the frame through a callback it supplies,
    so the registrar itself never names `PicturaMainWindow`. The composition root
    supplies a small `MenuRuntime` (refresh, status message, options-bar access)
    so handler bodies stay frame-free.
- Leave `PicturaMainWindow` as `QMainWindow` overrides, chrome/settings state,
  the composition root and signal wiring, the close entry point that forwards to
  `ImportSaveCoordinator`, plus the existing test hooks as thin forwarders so no
  file under `crates/pictura-app/cpp/tests/` changes. `registry()` remains a
  forwarding hook to the registrar's command table.
- Remove `frame.h` from the panel tree and from the menu translation units;
  forward-declare the panel/document types out of `frame.h` and split
  `frame_includes.h` so a panel or menu implementation edit no longer rebuilds
  the whole frame translation-unit set.
- Add a direct Qt Test per collaborator that constructs it without
  `PicturaMainWindow`, and a shell-seam guard with fixed ceilings (touched
  panel/menu files, collaborator headers, `frame.h` <= 760 lines,
  `frame_includes.h` <= 60 include lines) run by `scripts/verify-fast.sh`.
- No intended behavior change: self-test tokens and exit codes, session format,
  public C++ entry points used by `main.cpp`/the control server, and the existing
  222 `tst_*.cpp` suites, including the 159 that include `frame.h`, stay as they
  are.

## Capabilities

### New Capabilities

- `ui/main-window-collaborators`: the shell's owned-collaborator contract — what
  each collaborator owns, the include direction between them and the frame, the
  requirement that each is constructible without the frame, and the fixed-ceiling
  guard that keeps panel and menu code off `frame.h`.

### Modified Capabilities

- None. The shell's user-visible behavior and the existing application-shell
  requirements do not change.

## Impact

- **C++ shell** (`crates/pictura-app/cpp/`): new
  `document_registry.{h,cpp}`, `panel_host.{h,cpp}`,
  `import_save_coordinator.{h,cpp}`, `menu_registrar.{h,cpp}` plus concern
  siblings; the 45 `frame*.cpp` translation units are regrouped; panel TUs and
  `menu_*.cpp`/`menu_registrar.cpp` stop including `frame.h`, and no
  `frame_menus*.cpp` remains; `frame_includes.h` is split; `frame.h` drops to
  <= 760 lines. New sources are registered explicitly in
  `cmake/sources/frame.cmake` and `cmake/sources/panels.cmake` (no globbing).
- **Tests**: four new Qt suites (`tst_document_registry`,
  `tst_panel_host`, `tst_import_save_coordinator`, `tst_menu_registrar`) and a
  guard self-test. No file under `crates/pictura-app/cpp/tests/` is edited; the
  panel test-support TU `panels/panel_column_test.cpp` may change with the host
  pointer.
- **Guards**: `scripts/check-shell-seams.py` with fixed ceilings (`frame.h`
  <= 760 lines, `frame_includes.h` <= 60 include lines), wired into
  `scripts/verify-fast.sh`.
- **Rust / bridge / PSD / session format**: unchanged. No dependency added.
- **Docs**: `docs/GLOSSARY.md` and `docs/dev/STATE.md` follow the new ownership in
  a separate `TASK-ALLOWS-DOCS` commit.
