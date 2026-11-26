# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `dc_menu_reference` | Run `python3 scripts/check-public-docs-commands.py`. | Every catalogue id with a path is in its menu page's hidden manifest, or allowlisted when it has no path. | `check-public-docs-commands::check_public_docs` |
| `dc_menu_shortcuts` | Open each menu page and compare a sample of rows with the running app's menus. | The shortcut shown beside each command matches the app. | manual: the shortcut is read from the running app's menus; the catalogue supplies the same strings, and a human spot-check covers rendering |
| `dc_disabled_commands` | Open each menu page's **Not yet available** list and compare it with the running app's greyed commands. | Every greyed command appears there and no enabled command is listed as unavailable. | manual: greyed state comes from the running app; the catalogue's implemented flag is the checked source |
| `dc_preferences_reference` | Run the guard, then open `public_docs/preferences.md`. | Every Preferences page name is present; disabled pages are marked; Performance links to its detailed page. | `check-public-docs-commands::check_public_docs` |
| `dc_status_reference` | Run the guard, then open `public_docs/status-bar.md`. | Every readout menu label is present and explained. | `check-public-docs-commands::check_public_docs` |
| `dc_view_extras` | Run the guard, then open `public_docs/menu-view.md`. | Each Show extra has a description and its image file exists. | `check-public-docs-commands::check_public_docs` |
| `dc_command_guard` | Run `python3 scripts/check-public-docs-commands.py --self-test` and a temporary tree with a missing id. | The self-test passes; the temporary tree fails naming the id and page. | `check-public-docs-commands::self_test` |
| `dc_command_guard_allowlist` | Run the self-test and a temporary tree with an internal id and no allowlist entry. | The self-test passes; the missing entry fails, and a reason-less entry fails. | `check-public-docs-commands::self_test` |
| `dc_guard_ci` | Inspect `.github/workflows/guards.yml` and `scripts/verify-fast.sh`. | Both run the guard's self-test and the guard. | `check-public-docs-commands::check_public_docs` |
