# Design

## Context

See proposal.md for the motivation. Every number below comes from one machine:
16 CPUs, 62 GB, GCC with mold, sccache, Ninja, `-j12`, and the RelWithDebInfo
development configuration with the `dev-app` Cargo profile. The tree is
`ae831dc1` (after #1050), measured on 2026-10-30. The machine's load average
varied between 3 and 15 during the runs, so the edit-loop numbers carry noise.
The steps are listed in tasks.md section 1, so the **after** run repeats them
exactly.

### Baseline

| Scenario | Value |
|---|---|
| Cold C++ build (`SCCACHE_RECACHE=1`) | 535 s wall |
| ↳ shell TUs | 326 TUs, 1,746 CPU-s, median 4.9 s |
| ↳ `tst_*.cpp` | 179 TUs, 1,610 CPU-s, median 8.6 s |
| ↳ Qt Test AUTOMOC + `mocs_compilation` | 446 CPU-s |
| ↳ self-test TUs | 37 TUs, 258 CPU-s (`selftest.cpp` 40 s) |
| ↳ Qt Test links | 179 links, 16 CPU-s, about 0.1 s each |
| Warm rebuild, every object from sccache (725/725 hits) | 28 s wall |
| Bridge-file body edit (`type_tools.rs`) | 52 s wall |
| Non-bridge app edit | 10 s wall |
| `pictura-core` edit, dev-app | 47 s wall, 6 dependent crates rebuilt |
| `pictura-render` edit, dev-app | 7.5 s wall |
| `pictura-core` edit, `cargo build --profile dev-app -p pictura_app` (task 1.3) | 7.1 s wall; the slowest units are `pictura_app` 3.6 s, `pictura-filters` 2.3 s, `pictura-render` 2.0 s |
| `pictura-render` edit, same (task 1.3) | 5.9 s wall: `pictura_app` 3.0 s, `pictura-render` 2.7 s |
| `pictura-core` edit, `--release` (task 1.3) | 46.8 s wall: `pictura-filters` 33.7 s, `pictura-render` 21.2 s, `pictura-codec` 14.8 s, `pictura_app` 11.7 s |
| `pictura-render` edit, `--release` (task 1.3) | 19.8 s wall: `pictura_app` 11.4 s, `pictura-render` 8.2 s |
| `libpictura_shell.so` | 425 MB |
| Qt layer, `ctest -j4` | 249 s wall, 937 s summed over 2,433 cases |
| `initTestCase` total / median | 150 s / 0.8 s |
| Slowest single entry | `tst_ui_scale`, 57 s |
| `QTest::qWait(` calls in `crates/pictura-app/cpp/tests/` | 183 (104 of them `qWait(50)`); none in the self-test |

The six suites that build a window in each test function account for about
350 s of the 937 s:

| Suite | Cases of 1–5 s | Time |
|---|---|---|
| `tst_floating_workspaces` | 38 | 73 s |
| `tst_document_float` | 37 | 70 s |
| `tst_workspaces` | 28 | 57 s |
| `tst_import_pdf` | 31 | 56 s |
| `tst_document_arrange` | 28 | 55 s |
| `tst_document_windows` | 24 | 43 s |

### Window construction profile

The profile samples the main thread of `tst_workspaces` with gdb on `SIGALRM`
every 50 ms. Of 1,329 samples, 952 fall inside `PicturaMainWindow()`.

| Share | Where |
|---|---|
| 55% | `applyBrightness` → `Theme::apply`. It calls `QApplication::setStyle(new …)`, `setPalette` and `setStyleSheet` after every widget exists, which re-polishes the whole tree (`frame.cpp:186`, `theme.cpp:794`). |
| 14% | `applyFreshSessionDefault` → `regroupAndApply`. It builds the default panel groups, then harvests and rebuilds them. |
| 13% | `buildTools` → `OptionsBar`, which builds one page for every tool. |
| 13% | `buildPanels`, all 23 panels. |
| ~5% | everything else |

## Goals / Non-Goals

**Goals**

- Cut the Qt layer's wall time by at least 3× on the reference machine.
- Bring a bridge-file body edit down to the cost of a non-bridge app edit.
- Make every compile-pipeline change measurable as kept or skipped.

**Non-Goals** (each one measured and declined)

- **Lazy options-bar pages.** They are 13% of construction, and making them
  lazy changes behaviour in several places:
  - `OptionsBar::pageForTest` is `const` and maps the stack index to the tool
    enum (`options_bar.cpp:112-116`, `:1179-1182`). Tests reach unselected pages
    through it, for example
    `tst_options_bars_parity::toolIconSeparatorOnEveryPage` (`:121-122`), which
    walks every page without selecting the tool, and `tst_warp_text`
    (`:197`, `:210`, `:223`).
  - `setAlignEnabled`, `setRefineEdgeEnabled` and `setWarpTextEnabled` update
    only the buttons that already exist (`options_bar.cpp:582-591`,
    `options_bar_move.cpp:182`). A page built later would start in the wrong
    enabled state.
  - `updateStackPolicies` (`options_bar.cpp:1166`) keeps every page's vertical
    policy Preferred, so the bar's height is the tallest page's. Building pages
    lazily could change the bar's height, which users would see.
  - 29 Qt suites and the self-test find option widgets by name across the
    window (for example `selftest_ui_persistence.cpp:153`,
    `selftest_annotations.cpp:99`).
  - `options_bar.cpp` is 1,191 lines against the 1,200 cap and is not
    allowlisted, so the change would need a split as well.
- **Lazy panel creation.** Panels are 13% of construction. `regroupAndApply`
  harvests real widgets by object name, so it would need host widgets, and
  45 suites reach panels through 220 `findChild` call sites.
- **Linking each test only once** (#1043 D). The tests already link the Rust
  library only through `pictura_shell` (PRIVATE in the shared configuration),
  and a test link takes about 0.1 s with mold. Relinking 179 tests costs about
  12–16 CPU-s per Rust edit.
- **Splitting bridge generation across crates** (#1043 C.2). cxx-qt-build wipes
  its export directory on each `build()` and fixes one library name per crate.
  66 bridges include `cxxqt_object.cxxqt.h` and use `PictureView` state that
  would have to move to a base crate. Declined in favour of C.1.

## Decisions

### 1. Apply the theme once, early, and compare against the live application

`Theme::apply(family, level)` builds the palette and the style sheet for the
requested family, level and the current UI scale. It returns without touching
anything (style, palette, pixmap cache, palette generation) when all of these
hold:

- `QApplication::style()` is the theme's `SpinArrowStyle`;
- `qApp->palette()` equals the generated palette;
- `qApp->styleSheet()` equals the generated style sheet.

It still records the family and level. The comparison reads the application's
current state instead of a private record of the last application. A private
record goes stale when something outside the theme changes the application. One
suite does exactly that: `tst_keyboard_shortcuts.cpp:438` applies the theme,
and `:465` clears the style sheet with `qApp->setStyleSheet(QString())`. With a
private record, the next `Theme::apply` with the same inputs would skip, and
that process would keep an empty style sheet.

The UI scale reaches the comparison through the style sheet: the metrics and
font sizes in the sheet are scaled (`theme.cpp:549-563`, `:750+`). So
`setUiScalePercent` (`frame_ui_scale.cpp:126-148`) still re-applies, because a
new scale produces a different sheet.

The constructor applies the theme at one pinned place. It goes right after
`Theme::setUiScalePercent` and `applyBundledUiFont` and the window's
`setFont(QApplication::font())` (`frame.cpp:139-144`), because the style
sheet's font sizes are resolved from the application font. It also goes before
`buildMenus`, `buildPanels` and `buildTools` (`frame.cpp:156-158`). The family
is resolved at `frame.cpp:136`, so everything the call needs is known there. The
document tabs, the panel column and the splitter exist earlier
(`frame.cpp:56-97`, before the session is read at `:108`). They are few, and
the requirement names the menus, panels and tool controls. The later
`applyBrightness(state.brightnessLevel)` (`frame.cpp:186`) becomes a no-op on
the theme and still runs `applyWorkspaceCanvasColor`.

*Alternatives:*

- Keep the late call and only add the idempotence check. That helps the second
  and later windows in a process, but app startup and each suite's first window
  still pay for the re-polish.
- Swap `QApplication::setStyle` for a per-window style. That changes the
  look-and-feel contract.

Qt also re-styles a widget when it is reparented under an application style
sheet, and building the panels reparents them into their groups. So widgets
still receive style changes during construction after this change. The test
for the ordering therefore checks that the application style sheet is already
the restored theme's when the layers panel and the options bar build their
contents, not that no style change occurs.

*Risk, fonts:* `frame_ui_scale.cpp:137-139` notes that applying the theme
"recreates the style, which resets widget fonts". Today, `buildDocumentTabs`
(`frame.cpp:76`, `frame_build.cpp:830`) and `buildStatusBar`'s
`bar->setFont(footerFont)` (`frame_build.cpp:848`) run before the late apply.
Moving the apply earlier can change which explicit fonts survive.
`tst_theme_apply::chromeFontsSurvive` pins the result: the tab bar and the
status bar keep the fonts they were given. If the test shows that today's
window depends on the reset, the difference goes in this section before the
change lands.

*Measured side effect (task 12.2):* the column's splitter shares out its height
by the groups' size hints. Before, the Adjustments panel computed its size
hint under plain Fusion metrics and was restyled afterwards. Now it is built
under the final style sheet, so its hint is the styled one. On a 1100×700
first-run window the Adjustments group grows from 114 px to 122 px, and the
Color and Layers groups shrink from 220 px to 216 px and from 233 px to
229 px.

`tst_layer_target::clickingEmptySpaceClearsCurrentRow` assumed a fixed 10 px of
empty space below the last layer row, and it was passing with 4 px to spare.
It now asserts that empty space exists and clicks in the middle of it. This
is a weakened assertion: the required margin drops from 10 px to 2 px, because
the band's height follows the panel's share of the column. Bisected: disabling the early apply restores the old heights, while disabling
the staged first-run groups (§2) changes nothing.

*Audit (task 3.3, done):* a grep of the suites, `qt_test_support.h` and
`selftest*.cpp` for `qApp->setPalette`, `qApp->setStyleSheet`,
`qApp->setStyle`, `QApplication::setStyle` and `QApplication::setPalette` finds
one hit outside `tst_theme_apply`: `tst_keyboard_shortcuts.cpp:465` clears the
style sheet. The live comparison handles it: the cleared sheet differs from the
theme's, so the next application runs in full.
`tst_theme_apply::reapplyAfterExternalStyleSheet` pins that.

The installed style is tracked with a `QPointer` to the style
`QApplication::style()` returned after the last full application, not by
checking the style's type. Once an application style sheet is set, Qt wraps
the theme's style in its style-sheet style, so `QApplication::style()` is not a
`SpinArrowStyle`. A replaced style changes the pointer or deletes the tracked
one.

### 2. Build the first-run layout once

`applyFreshSessionDefault` (`frame_workspaces.cpp:153-166`) today lets
`buildPanels` create the default groups and then calls
`regroupAndApply(workspacePresets("Essentials"))`. That harvests every panel
(`frame_workspaces.cpp:214+`) and rebuilds the groups. On a fresh session,
`buildPanels` still creates its eight default groups but does not attach them
to the column. It returns them as staged groups, and the constructor passes
them through `applyFreshSessionDefault` to `regroupAndApply`, which harvests
them first and deletes them. The default groups are then never inserted into
the column, laid out and taken apart again. The Essentials description stays
the single source of the arrangement, and workspace switching is unchanged.

*Alternative, declined at implementation:* have `buildPanels` place each panel
straight into the group the Essentials description names and skip the regroup.
That would turn the 430-line `buildPanels` into a data-driven builder for a
saving close to the staged version's: both skip the column insert, layout and
teardown, and the staged version only adds one extra reparent per panel. The
staged groups travel through return values rather than a member, because
`frame.h` is at its 1,200-line cap. `tst_workspaces::freshLayoutMatchesEssentialsApply` pins the
result: the first-run arrangement equals the arrangement applying Essentials
produces.

### 3. CTest parallel level

- Local default (`scripts/test-report.sh:55`): `max(1, ⌊0.8 × nproc⌋)`. The 80%
  cap matches the compile limit in AGENTS.md and leaves headroom for the two
  self-test runs that go alongside the Qt layer. On the 16-CPU machine that is
  12 suites, plus two self-test processes, for 14 offscreen processes.
- CI default (`ci.yml:235`, `qt-headless`): `nproc`. The runner runs nothing
  else at that point, and a 4-vCPU `ubuntu-24.04` runner keeps its current level
  of 4. With the 80% rule it would drop to 3.
- `CTEST_PARALLEL_LEVEL` overrides both.

`tst_ui_scale` is shardable. Its `init()` removes the session file, resets the
scale and builds a fresh window for each function (`tst_ui_scale.cpp:58-70`),
and its `cleanup()` destroys the window. It moves from `QTEST_MAIN` to
`pictura::test::shardedMain` and joins `PICTURA_QT_TEST_SHARDS`.

*After (task 12.3):* at the new default of `-j12` the first three runs took
71 s, 71 s and 72 s of wall time (205/205 entries passed), against the
249 s baseline at `-j4`. Summed case time went from 937 s to 804 s. The
`initTestCase` median rose from 0.8 s to 1.18 s because 12 suites now compete
for the CPU instead of 4.

At that point the longest entries were `tst_ui_scale_1` at 21.4 s and
`tst_print` at 19.4 s, both over a quarter of the 71 s wall
(`qth_longest_entry`). Every function of `tst_print` and `tst_edit_refresh`
passes when run alone and in every share, so both moved to `shardedMain` and
joined the shards. `tst_edit_refresh` shares one window, so it was reviewed
like the six suites in §4, where its row is.
`tst_ui_scale`'s two slowest functions (7.7 s and 6.2 s) fell into the same
share of four, so it now has eight shares through a per-suite
`<suite>_SHARD_COUNT` in `tests/CMakeLists.txt`.

Two runs with 215 entries then took 77 s and 72 s; the 77 s run was noise.
The longest entry was `tst_loading_tab` at 17.1 s, 23.7% of the 72 s run, so
the bound held only narrowly. `tst_loading_tab` shares one window whose
`init()` and `cleanup()` reset everything its functions read (§4 table), so it
also moved to `shardedMain` and joined the shards of four.

### 4. Shared windows, suite by suite

The candidates are the six suites listed above. All six are already registered
in shards. `tst_document_float`, `tst_document_windows` and
`tst_document_arrange` build their window in `init()`; `tst_document_arrange`
also builds extra windows to test reloads (`:720`, `:748`).
`tst_floating_workspaces`, `tst_workspaces` and `tst_import_pdf` build one per
function or helper. For each suite:

1. List the state its functions read: documents, workspace store, session
   file, float windows, preferences.
2. Write the reset in `init()`.
3. Run each function alone and each shard.

A suite whose reset cannot be shown complete stays as it is, and the decision
is recorded in a table here. Do this after Decision 1, because cheaper
construction shrinks the payoff, and a suite may not be worth the risk once
its windows cost 0.4 s.

Running "each function alone" needs a harness fix first. Without
`PICTURA_TEST_SHARD`, `shardedMain` (`qt_test_support.h:126-155`) appends every
test function to argv. So `tst_x someFunction` runs the whole suite. The helper
that builds the argument list leaves argv as it is when argv already names test
functions. `tst_test_support` pins that, without needing a window.

The comment in `tests/CMakeLists.txt:191-193` ("A suite that shares its window
… keeps one entry") changes to the rule in the spec: a suite may share a window
and stay sharded when its `init()` reset makes every function independent.

The decisions (task 7.2; the last three rows are task 7.4, for the suites that
joined the shards in §3). In the three document suites, persisted state
(session file, workspace store, tool options, preferences) already carried
from one function's window to the next: each suite held one
`ScopedStateHome` for all its functions, and each window saved the session as
it closed. A shared window adds only the in-memory state below.
`tst_import_pdf` held a state home per function, but no function reads
persisted state after the frame is built. The reset runs in `cleanup()`, which QTest also runs
after a failed function, and `init()` checks it: no documents open, the frame
at 1100×700 and active again.

| Suite | State its functions read | Decision | Reason | Functions that build their own window |
|---|---|---|---|---|
| `tst_document_float` (37) | documents, tabs, floats (child and top-level), the dock line, the docking preference, screen mode, hidden panels, active tool, active workspace (one switch to Painting), the dock-drag hook and the static float overrides, window size and activation | share | Closing every document removes its floats and tile panes; the docking preference is turned back on, screen mode set to Standard, panels shown, Essentials re-selected, Move chosen, the dock line hidden, and pending deletes flushed. Brush size and other tool options were already shared through the session. | none |
| `tst_document_windows` (24) | documents, tabs, floats, the tab menu, active tool, focus, the open delay, the URL opener hook, the unsaved-prompt choice | share | Closing every document (cancelling loading ones first) and clearing the open delay and the URL hook restore what a fresh frame has; each function that needs focus sets it. | none |
| `tst_document_arrange` (29) | documents and their extra windows, floats, Tile, both window preferences, window size, active tool, the open delay, the unsaved-prompt choice and counter | share | The last tile pane leaving ends Tile, the preferences go back on through the dialog, the size is restored in `init()`, and the prompt counter is only read as a difference. | `prefsPersist` (a reload over the shared state home), `recoveryReopensAsTabs` (a crashed and a recovering frame in their own state home) |
| `tst_floating_workspaces` (38) | panel columns, rail mode, group and column floats, flyouts and their widths, dynamic columns, float geometry, workspace store, session file | keep | Re-applying Essentials restores the arrangement but not the sizes: `freshLayoutMatchesEssentialsApply` leaves width, height and float geometry out on purpose, and these functions measure them. Six functions test a restart or a stored layout read at construction. | all |
| `tst_workspaces` (33) | workspace store (user workspaces, deleted presets), active workspace, panel groups, the session file, the menu and switcher | keep | A deleted preset stays reserved in the frame's store, and no public call restores it; about a third of the functions test first construction (fresh-session default, restart, a lost session) or the store alone. | all (the store-only functions build none) |
| `tst_import_pdf` (32) | documents, the import and PDF dialog drivers, the open delay, the status message, `PICTURA_GHOSTSCRIPT` (read on each open) | share | The frame is never shown; the import dialogs are built per open and keep no settings; closing every document and clearing the two drivers, the delay and the status message restore it. The loading-tab timers now use a local context object, so none fires into a later function. | none |
| `tst_edit_refresh` (19), reviewed after it joined the shards (§3) | documents and their extra windows, the active document, the Layers, Channels and Paths panels' contents, the registry, Layers refresh, dock-retarget and window counters | share (already one window since `initTestCase`) | Every function opens its own documents and reads the counters only as differences from a baseline it takes after a settle. `cleanup()` closes every document, which closes their windows, and now also flushes the pending deletes. Each function passes alone and in every share. | none |
| `tst_print` (4), reviewed after it joined the shards (§3) | the process-wide last print settings, a print dialog per function, one frame in `menuCommands` | per function (no shared window) | The three dialog functions set the last print settings before they build their dialog; `menuCommands` reads none and builds and drops its own frame. Each function passes alone and in every share. | `menuCommands` |
| `tst_loading_tab` (26), reviewed for `qth_longest_entry` (§3) | documents and loading tabs, the open delay, the status message, the override cursor, the import dialog driver, held workers | share (already one window since `initTestCase`) | `init()` sets the open delay and clears the status message; `cleanup()` restores the override cursor, cancels every open, closes every document and clears the driver. A cancelled worker's late result is discarded, and the negative-window waits let it land inside its own function. | none |

### 5. Condition waits

Each `QTest::qWait(n)` falls into one of three classes:

- **condition:** becomes `QTRY_VERIFY` / `QTRY_COMPARE` / `QTest::qWaitFor` on
  the condition the test checks next;
- **event flush:** becomes `QTest::qWait(0)` or
  `QCoreApplication::sendPostedEvents`, for example for `deleteLater` or a
  zero-length timer;
- **negative window:** stays, with a comment on its line or the line above.

Before changing a wait, confirm its behaviour under `QT_QPA_PLATFORM=offscreen`.
The Rust test `build_config::qt_test_waits_are_justified` scans
`crates/pictura-app/cpp/tests/*.cpp` and `*.h`. It fails on any `qWait(…)`
whose argument is not the literal `0` and that has neither a `//` comment
after the call on its line nor a comment-only line right above it. The argument can be an expression such as
`qWait(kOpenDelayMs + 150)`, which appears 4 times. It runs
under nextest, so `verify-fast.sh` and CI enforce it.

#### Inventory

Before the change, `grep 'qWait('` over `crates/pictura-app/cpp/tests/*.cpp`
and `*.h` found 183 calls in 22 files. One of them was already `qWait(0)`.

| Class | Count | Replaced | Kept, with a comment |
|---|---:|---:|---:|
| condition | 47 | 16 | 31 |
| event flush | 106 | 103 as `qWait(0)`, 102 of them changed (1 already was) | 3 |
| negative window | 30 | 0 | 30 |
| **total** | **183** | 119 | 64 |

The 16 replaced condition waits now poll the assertion that follows them:

- 4 canvas pixel probes in `tst_transform_skew` (`QTRY_VERIFY2` on `grab()`);
- 2 ACR preview renders behind the dialog's 30 ms throttle (`tst_acr_dialog`);
- 2 spinner ticks and a middle-click close (`tst_loading_tab`);
- 2 top-level resizes (`tst_layers_panel_fixes`);
- 1 each: the Background-hide checkerboard (`tst_canvas_backdrop`), the mask
  edit target (`tst_layers_mask_commands`), the dialog that the 700 ms timer
  rejects (`tst_paragraph_styles_panel`, `qWaitFor`), the canvas becoming
  visible (`tst_selection_preview`), and the pattern actions that disable
  after a document switch (`tst_edit_refresh`).

The 31 kept condition waits wait for a debounce timer that the production
code does not expose. Twenty-seven are `tst_edit_refresh` settles that let
`panelRefreshTimer_` (120 ms) and the dock retarget drain before the test
records a baseline count. Polling for the first refresh would leave a second
one pending, so the baseline would be wrong. The other four:
`tst_canvas_backdrop` (new-document settle), `tst_paragraph_styles_panel`
(refresh between the two clicks), and `tst_selection_preview` (two waits for
the paint that follows an ants tick).

The 103 event flushes follow a `show()`, `refresh()`, `setRailMode()`,
synthetic mouse move or layout pass. Every one of those runs synchronously
or posts one zero-length callback: the layers panel's queued refresh and the
transform preview, which computes inside `transform_move`. One pass of
`qWait(0)` delivers them. The three kept flushes are in `tst_offthread_jobs`,
whose `TimerProbe` measures event-loop gaps, so the new-document startup
must be fully drained before the probe starts.

Negative windows, kept as fixed delays because the asserted event must not
happen within the window:

- `tst_edit_refresh.cpp`:334, 342, 372, 382, 393, 402, 411, 465, 486, 538,
  556, 579, 617, 635, 667, 696, 723: no second Layers refresh, dock
  retarget, or registry refresh lands after the 120 ms debounce (465: no
  retarget while a filter commit is running; 667: the contour is not
  retraced).
- `tst_layers_preview_coalescing.cpp`:269, 296: a fast round trip neither
  previews nor records a state. 368, 404, 453: a cancelled preview never
  commits. 477: 40 ms drag ticks inside the 120 ms preview delay. The spacing
  is the input under test.
- `tst_loading_tab.cpp`:467, 512, 532, 666: a cancelled open's held worker,
  arriving `kOpenDelayMs` later, adds no document and no status message.
- `tst_document_windows.cpp`:290: a cancelled close leaves the tab open.
- `tst_pen_tools.cpp`:349: another anchor adds no second Work Path row.
- `tst_paragraph_styles_panel.cpp`:377: the font popup stays open after the
  click.

After the change, 64 calls have a non-zero argument (31 condition, 3 event
flush, 30 negative window). Each one has a `//` comment after the call or on a
comment-only line right above it.

### 6. Declarations-only bridge files

build.rs hands 77 files to cxx-qt. `src/cxxqt_object.rs` already holds only its
bridge, `mod` declarations, `pub use state::PictureViewRust` and `#[cfg(test)]`
module declarations, so it needs no move. The other 76 do:

- the 75 listed under `src/cxxqt_object/`;
- `src/decode_bridge.rs`, the only other path outside `cxxqt_object/`, which
  has a body at `:16`.

For each one, `foo.rs`, the bodies, helpers and inline `mod tests { … }` move to
a sibling implementation module, or into an existing `impl_*` module where one
exists. `foo.rs` keeps:

- the bridge module;
- its `mod` declarations for child modules, some of which are bridges themselves
  (`type_tools.rs:415-418` declares `warp` and `fonts`);
- `#[cfg(test)] mod x;` declarations of out-of-line test files;
- `use super::foo_impl::{…}` and the existing `pub use` items.

The child module files stay where they are, so build.rs's file list does not
change. cxx resolves `extern "Rust"` functions through `super::`, and a private
`use` in the parent is visible to the child module, so the generated code is
unchanged.

The Rust test `build_config::bridge_files_hold_declarations_only` guards the
rule. It reads the paths from build.rs's `.files([...])` list, and for each
one it fails on:

- a listed file that holds no bridge;
- a `fn` with a body outside the bridge module;
- a `mod` with an inline body outside the bridge module.

It reuses the module-extraction helper that `bridge_modules_end_at_column_zero`
already uses.

*Alternative:* a build-script short-circuit that hashes the bridge modules and
skips `CxxQtBuilder::build()`, replaying the previous directives. Rejected:
cxx-qt-build prints its directives straight to stdout, so replaying them
depends on cargo's internal output files.

*As built:*

- **Layout.** Each of the 76 files keeps its bridge and moves everything else to
  a child module `<name>/imp.rs`, re-exported with `use imp::*;` at the
  original effective visibility. Four out-of-line test files moved under
  `imp/`.
- **Guard.** The guard accepts grouped `use` items, any `pub(...)`
  visibility and an empty `unsafe impl`, and skips string literals inside
  attributes.
- **Build.** The first compile of the split had no errors and no new warnings,
  and clippy is clean.

*After (task 12.3):* a body edit in `type_tools/imp.rs` took 23.9 s of wall time
(load 6), against 52 s for the same edit in `type_tools.rs` before. The build
script did not rerun, and no generated C++ was recompiled. The C++ work was
the relink of the shell and the test executables, 11 CPU-s.

### 7. Compile-pipeline experiments, each gated by the measurement rule

The rule is the modified "heaviest C++ translation units" requirement. A change
is kept only when neither the cold build nor the warm sccache rebuild gets
slower and at least one gets faster, or the trade is recorded here. A kept flag
change applies to the optimized development configuration only (`BUILD_TESTING`
on with Release or RelWithDebInfo). Debug and packaging keep their settings.
Each experiment is run cold (`SCCACHE_RECACHE=1`) and warm (objects deleted,
cache kept), before and after.

- **PCH.** `target_precompile_headers(pictura_shell PRIVATE <pch.h>)` with the
  common QtCore/QtGui/QtWidgets headers and common std headers. As built
  (`pictura_pch.h`), project headers and the generated cxx-qt headers stay out,
  so an edit to one does not rebuild the PCH and every unit that uses it. The
  PCH is enabled only for GCC and Clang, the compilers the `-Og -g1` flags
  apply to; it was measured with GCC only, so AppleClang and MSVC builds compile
  without it. The Qt Test targets cannot
  `REUSE_FROM pictura_shell`, for two reasons:
  - their flags differ (`-g1`);
  - the development shell is a shared library built `-fPIC`, while the test
    executables are built `-fPIE`.

  They get their own PCH target, `pictura_test_pch`, that the tests
  `REUSE_FROM`. Risks:
  - sccache may not cache GCC compiles that use a `.gch`. If the warm rebuild
    goes from 28 s to several minutes, PCH fails the rule unless it is
    restricted to builds without a compiler launcher.
  - A PCH can hide a missing `#include`. The `qt-headless` CI job configures
    with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON`, so CI compiles every source
    without the PCH. CI leans on sccache, not PCH, anyway.
- **Test AUTOMOC.** The test sources `#include "tst_x.moc"`. Turn `AUTOMOC` off
  for the test targets and run `qt_wrap_cpp`-style moc on the one source, or
  keep AUTOMOC with no `mocs_compilation` TU. Measure the autogen and
  `mocs_compilation` CPU before and after.
- **`-Og -g1 -gsplit-dwarf`.** Optimized development configuration only. It is
  added as a target-level option ahead of the per-source options, so
  `filter_commands.cpp` still ends with `-O1` and the Qt Test units with `-g1`
  (`bp_cheap_tu`, `bp_test_debug`). `command_tree.cpp`, which the earlier
  requirement also named, no longer exists; this change's delta drops it. This experiment also
  needs the Qt layer's wall time, because `-Og` C++ may slow the GUI suites.

Found at the first build (task 12.1):

- GCC refuses a precompiled header for any unit compiled with
  `-gsplit-dwarf` (`not used because '_gsplit_dwarf_path' defined as …`),
  because each unit names its own `.dwo`. The two experiments exclude each
  other, so split DWARF was measured as its own configuration (below) and then
  removed. When a PCH is rejected, GCC falls back to including the
  header as text, which costs more than no PCH at all.
- The self-test executable takes no PCH. Its sources rely on not seeing Qt's
  keyword macros (`selftest_report.h` declares a member function named `emit`),
  and the umbrella Qt headers define `emit`.

Measured at `-j12` with load averages of 12 to 17 (task 12.4). Each configuration enables one experiment over a
"none" configuration with all four off. Cold builds recompile every C++ unit with `SCCACHE_RECACHE=1`;
warm builds serve every unit from sccache. CPU is the summed time of the compile steps in `.ninja_log`.

| Experiment | Cold wall (CPU) | Warm wall | Qt layer | Verdict |
|---|---|---|---|---|
| none | 258 s (3,009 s) | 23.4 s | 72 s | baseline for this table |
| PCH | 210 s (2,337 s) | 44.7 s | — | **kept, trade recorded** |
| Direct moc for the Qt Test targets | 263 s (3,051 s) | 24.0 s | — | **skipped**: no measurable gain |
| `-Og -g1` | 205 s (2,386 s) | 25.6 s | 74 s | **kept** |
| `-gsplit-dwarf` | 270 s (3,137 s) | 25.9 s | — | **skipped**: slower cold; excludes the PCH |

*Final configuration* (PCH and `-Og -g1` together, the direct-moc change
reverted; load average 14–15):

| Check | Result |
|---|---|
| Cold C++ build | 151 s of wall time and 1,666 CPU-s, against 258 s and 3,009 CPU-s for "none" (the 535 s in the baseline table also included a 215 s cargo step) |
| Warm rebuild | 44.3 s |
| `ninja -t clean`, then a build (task 12.6) | succeeds and regenerates `initializers.o` |
| Qt layer | 74 s, 215 of 215 entries pass |
| `libpictura_shell.so` | 240 MB, down from 425 MB |
| `-DBUILD_TESTING=OFF` and Debug configures | no `cmake_pch` and no `-Og` in any command |
| A build with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` | compiles every source (293 s, mostly cache misses) |

*PCH trade.* The cold build gets 19% faster in wall time and 22% in CPU. The
warm rebuild nearly doubles: replaying a full tree from sccache now also
replays the large `.gch` files, and every unit's cache entry carries them.
That warm case is a fresh worktree or a full clean with a shared cache. The
common development case is an edited project header. The header is not in the
PCH, so every unit that includes it is a cache miss, and each of those units
saves about 0.9 CPU-s with the PCH: about 13 s of wall time for `frame.h`'s 175
includers at `-j12`. The PCH is kept for that reason, and these numbers are
the record the requirement asks for.

`filter_commands.cpp` compiles at `-O1` and cannot use the PCH, which is built
at the target's `-Og`, so GCC read the header as text. It now has
`SKIP_PRECOMPILE_HEADERS`. Its `-O1` block now tests `PICTURA_DEV_OPTIMIZED`
instead of repeating the build-type match. The only difference is a
multi-config generator with a stray `CMAKE_BUILD_TYPE`, which used to get
`-O1` in every configuration and now gets none, like the `dev-app` profile.

*`-Og -g1`.* The warm rebuild moved 2.2 s and the Qt layer 2 s, both within the
spread of these loaded runs. The cold build is 20% faster. The measurement
switches are gone: the kept settings apply unconditionally in the optimized
development configuration, and the skipped ones were removed.

### 8. Clean rebuild

`build/cxxqt/` is written by the cxx-qt build script as a side effect that
cargo does not track. `initializers.o` is the output of the
`pictura_app_mock_initializers` custom command, whose command is `cmake -E true`
(`build/build.ninja:4733`). So `ninja -t clean` deletes it and nothing rebuilds
it, because cargo's fingerprint is still fresh. The fix gives that custom
command, or the Corrosion target, a dependency that makes the cargo step rerun
when the export directory is missing. Two candidate mechanisms:

- a byproduct declaration, so Ninja knows the cargo step produces it;
- a forced cargo rerun.

Neither fits: CxxQt.cmake owns the mock command, and cargo has no notion of
a byproduct. The fix uses a stamp the build script watches instead:

- CMake creates `<export dir>/pictura_app.regenerate` at configure time, if it
  is missing.
- The build script emits `rerun-if-changed` for the stamp. It does so only
  when the stamp exists, because a watched file that is missing reruns the
  script on every build.
- A `pictura_cxxqt_export_check` target, which `cargo-prebuild_pictura_app`
  depends on, runs `cmake/cxxqt_export_check.cmake` before each cargo step.
  The script touches the stamp only when `initializers.o` is missing.

After a clean, the next cargo step reruns the build script once and writes the
object again. In every other build the stamp is untouched and the script stays
fresh.

### 9. CI target cache

The `*rustcache` anchor (`ci.yml:95-98`) was shared by `lint`, `test`,
`gpu-parity` and `qt-headless`. It is split:

- `lint` and `test` get their own step with `cache-targets: "true"`, a
  `shared-key` per job, and
  `save-if: ${{ github.ref == 'refs/heads/master' }}`;
- `gpu-parity` and `qt-headless` keep `cache-targets: "false"` through the
  `*rustcache` anchor, now defined in `gpu-parity`. `qt-headless` builds its
  Rust through Corrosion into the CMake build tree, not `target/`, so
  rust-cache would not find it, and sccache already serves its C++.

Only a run on `master` saves, so no pull request, this one included, can see a
restore of the new caches. The pull request's CI shows the per-job keys and the
skipped save. The job wall times and cache sizes after the first `master` run,
against the 10 GB quota, are recorded in a follow-up issue. The quota is shared
with sccache's GHA backend, `install-qt-action`'s Qt cache and the ImageMagick
cache.

### 10. Engine-crate edits

Task 1.3 ran `cargo build --timings --profile dev-app -p pictura_app` and the
same with `--release`, after a one-line edit to `pictura-core` and then to
`pictura-render`, on the untouched tree (load average 16–21). The baseline
table holds the numbers.

Under `dev-app` the six crates that depend on `pictura-core` rebuild in about
3–4 s of incremental work each, and the whole edit costs 7 s. That is less
than the 10 s non-bridge app edit, so the dependents do not dominate the edit
loop. The release profile's 47 s is not on the development path: CMake
development builds use `dev-app`, and only packaging uses `release`. So no
`[profile.dev-app.package.<crate>]` override is proposed. A lower opt-level
would only slow the Qt layer for a saving of a few seconds. Task 12.5 confirms
this against the after-run numbers.

After the change (task 12.3), the CMake-driven `pictura-core` edit took 33 s
of wall time at a load of 8. The build script did not rerun, and the C++ work
was relinking: 11 CPU-s for the shell and the test executables. The rest is
Corrosion's cargo step plus load. The 47 s → 7 s gap came mostly from load
during the baseline. No profile override (task 12.5): the dev-app engine edit
is a few seconds of cargo work, and a lower engine opt-level would only cost Qt
layer time.

## Risks / Trade-offs

- **[Panel-menu hooks are lost on a regroup]** → Pre-existing on `master`,
  not changed here: `regroupAndApply` deletes the groups that carry the
  Actions, Character and Paragraph panel-menu hooks (`frame_build.cpp`), and
  the first-run layout has always gone through a regroup. Tracked in a
  follow-up issue.
- **[A test relied on the theme being reset by the next window]** → The live
  comparison in Decision 1. The full Qt layer must pass at `-j1` and at the new
  default level.
- **[Explicit chrome fonts change with the earlier apply]** → `chromeFontsSurvive`
  and the font note in Decision 1.
- **[More parallel suites make the offscreen tests flaky]** → Run the layer
  three times at the new default before landing it, and keep each suite's
  `TIMEOUT 120`.
- **[A shared-window suite carries state across functions]** → The
  per-function and per-shard scenarios in the spec, made runnable by the
  harness fix. A suite stays per-function when in doubt.
- **[The bridge move breaks generated code or hides a semantic change]** → Pure
  moves only, done file by file. `cargo clippy -D warnings`, the full Qt layer
  and the self-test are the check. No body changes in the same commit.
- **[PCH or split DWARF makes compiles uncacheable]** → The measurement rule.
  The experiment is dropped, or restricted to builds without a launcher.
- **[PCH hides a missing include]** → CI builds with precompiled headers
  disabled.
- **[CI cache eviction]** → Save on `master` only and measure the sizes, and
  drop target caching for a job whose cache exceeds its share.
- **[Commits are not built one by one]** → AGENTS.md compiles once at the end,
  so the per-step commits are not built individually. The final build, the
  full Qt layer and the self-test cover the series. A failure found there is
  fixed in the commit that caused it.

## Migration Plan

Implementation order: the measured wins first, the large mechanical diff in
the middle, the experiments last.

1. Theme once
2. CTest level and `tst_ui_scale` shards
3. Condition waits
4. First-run layout
5. Shared windows
6. Clean rebuild
7. Bridge split
8. Compile experiments
9. CI cache
10. Engine decision (measured in task 1.3)

Each step is a separate commit that can be reverted alone. There is no data or
format migration.

