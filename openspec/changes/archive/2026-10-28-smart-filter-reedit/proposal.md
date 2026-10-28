# Proposal: smart-filter-reedit

## Why

Review of #984 (PR #1005) found two CS6 behaviours missing from the Smart
Filters the Filter menu now applies: the Layers panel listed the chain
oldest-first, where CS6 draws the most recently added filter on top; and a
generic or Filter Gallery Smart Filter could not be re-edited (#997), where
CS6 reopens its dialog on a double-click, first noting that the filters stacked
above it will not preview during the edit.

## What Changes

- The Smart Filters rows list the newest filter on top; each row still names
  its chain index, so the eyes and menus act on the right filter.
- Double-clicking a Smart Filter row, or Edit Smart Filter…, re-edits a
  generic filter with a dialog or a Filter Gallery stack: CS6's notice when
  filters are stacked above it (with Don't show again), then the dialog
  prefilled; the preview shows the chain up to the edited filter, and OK
  replaces the entry in place, keeping its position, enable flag, blend
  options, and the filter mask.
- `pictura-codec` gains `replace_smart_filter`, `pictura-render`
  `replace_smart_filter_at`.

## Capabilities

### Modified Capabilities

- `imaging/smart-filter-apply`: generic and gallery Smart Filters re-edit.
- `ui/smart-filter-context-menus`: Edit Smart Filter… covers generic and
  gallery filters, and a double-click does the same.
- `ui/layers-panel`: Smart Filter rows list the newest first.

## Impact

`pictura-codec` (`pictura_raw`), `pictura-render` (`smart_filter_apply`),
`pictura-app` (`smart_apply`, `layers_smart_filters`, the Layers panel, the
Filter Gallery dialog, `frame_menus_filter.cpp`, the session store),
`ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`. Closes #997.
