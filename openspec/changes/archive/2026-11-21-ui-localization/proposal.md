# Proposal

## Why

Kooka Pictura currently shows its interface in English, and shared numeric
fields expect a period even when the system uses a comma decimal separator.
[#642](https://github.com/KookaStudio/KookaPictura/issues/642) adds a way for
users to run the app in their language and enter measurements in familiar form.

## What Changes

- Bundle German and Finnish Qt catalogs for app-owned interface text across
  menus, tools, the Options bar, dockable panels, and dialogs. Give both
  languages the same coverage and preserve stable menu, panel, and option IDs.
- Add a language selector and **Import Translation** in **Preferences →
  Interface**. Default to the system UI language and allow explicit English,
  Deutsch, Suomi, and imported community catalogs. A changed selection applies
  at the next launch after the choice is saved successfully. Offer a restart
  directly from the Language row while a change is pending.
- Use the system numeric locale in shared numeric fields, ruler values, and
  Info measurements, including custom scales and calibrated ruler lengths. Keep
  numeric values canonical across document saves.
- Track RTL interface mirroring and bundled RTL languages in
  [#1488](https://github.com/KookaStudio/KookaPictura/issues/1488).

## Capabilities

### New Capabilities

- `ui/localization` covers bundled and imported UI catalogs, language
  selection, and startup fallback.

### Modified Capabilities

- `ui/numeric-fields` adds locale-aware validation, editing, and display.
- `ui/rulers-and-guides` adds locale-aware ruler labels and measurement text.
- `ui/info-histogram-panel` localizes the Info panel's measurement values.

## Impact

The Qt shell's startup, Preferences and its typed store, menus, tool controls,
panel tabs and contents, dialogs, and numeric displays are affected. Qt Linguist becomes a build
dependency for compiling bundled catalogs. Translation changes affect
presentation only. Command identities, shortcuts, document schemas, and
machine-readable outputs retain their current formats.
