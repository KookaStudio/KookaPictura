# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in the delta spec, naming the automated Qt test in the Coverage column, and verify the row count equals the `#### Scenario:` count
- [x] 1.2 Add the planned Qt test names to the plan before implementation so each later group can check its own rows off

## 2. Menu rows and routing

- [x] 2.1 Add the full Character and Paragraph rows plus a checkable submenu-child helper to `rowsForPanel()`, keeping Close / Close Tab Group last
- [x] 2.2 Route `character.*` and `paragraph.*` ids in `PanelGroup::runPanelMenuAction()` to new `performPanelMenuAction()` methods
- [x] 2.3 Implement the Character handlers (style/OpenType toggles, Fractional Widths, System Layout, Reset Character) and the Paragraph handlers (hanging, composers, Justification, Reset Paragraph)

## 3. State sync

- [x] 3.1 Sync the Character rows' checked/enabled state from the buttons, `shown_`, and the OpenType feature support in `CharacterPanel::updatePanelMenu()`
- [x] 3.2 Sync the Paragraph rows' checked/enabled state from the checkbox, composer, and type-layer availability in `ParagraphPanel::updatePanelMenu()`
- [x] 3.3 Set the disabled Latin/East Asian Layout and Default Features rows to their CS6 default check state

## 4. Tests

- [x] 4.1 Add `characterMenuMatchesCs6`, `characterMenuCheckStates`, `characterMenuToggles`, and `characterMenuReset`
- [x] 4.2 Add `paragraphMenuMatchesCs6`, `paragraphMenuCheckStates`, `paragraphMenuToggles`, and `paragraphMenuReset`
- [x] 4.3 Add `menuDisabledRows` and `menuCloseEntriesLast`
- [x] 4.4 Update `fractionalWidthsMenu`, `justificationDialogRoundTrips`, and `justificationCancelWritesNothing` to use the live `PanelGroup` menu and `trigger()`

## 5. Verification and archive

- [x] 5.1 Build once with CMake and run `ctest --test-dir build -R '^tst_character_paragraph_panels$' --output-on-failure`
- [x] 5.2 Run `bash scripts/verify-fast.sh` with `TASK_ALLOWS_DOCS=1` and fix every failure
- [x] 5.3 Run `openspec validate --all --strict`, archive `type-panel-menus`, and validate again
