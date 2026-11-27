# Tasks

## 1. Test sources

- [x] 1.1 Write and register `tst_single_line_marquee` cases named in the test plan, plus `moveOutAndBack` and `sharedOptions` regressions.

## 2. Tool integration

- [x] 2.1 Register both tool IDs, catalog/flyout entries and handlers, preserving default shortcut assignments and rectangle/ellipse Shift+M cycling, including the fallback from either line tool.
- [x] 2.2 Route mode, feather and Refine Edge controls without Style or Anti-alias, and refresh shared options when switching tools.
- [x] 2.3 Commit clicked rows and columns through the existing mask, feather, combine and history pipeline; show the low-coverage warning while retaining selection and history.
- [x] 2.4 Integrate selection cursor, movement and context menus, distinguishing clicks from selection-move drags, including a drag returning to its start.

## 3. Integration and verification

- [x] 3.1 Review and update ROADMAP.md and PARITY.md for changed feature coverage.
- [x] 3.2 Build the completed implementation, run the planned Qt suite and applicable repository gates, and resolve any failures.
