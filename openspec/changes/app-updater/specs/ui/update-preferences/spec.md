# ui/update-preferences Specification

## Purpose

Defines the user-facing update surface: the startup banner, the Preferences >
Updates pane, the automatic-check opt-out, download progress and error states,
the manual-check menu entry, and the version and package-method readouts.

## ADDED Requirements

### Requirement: An available update is announced in a non-modal banner

When a newer release is available, the application SHALL show a non-modal
banner at the top of the main window. The banner MUST name the running and
available versions as an upgrade path and MUST offer **Release Notes** and
**Later**. It MUST offer **Update Now** when the detected package method can
self-update, and **Show Update Command** when the package manager owns the
installation. The banner MUST NOT steal focus from the document or a field.

#### Scenario: The banner shows the upgrade path [up_banner_version_path]

- **WHEN** the running version is `0.3.0` and `0.4.0` is available
- **THEN** the banner text contains `0.3.0 → 0.4.0` and the actions for the
  detected package method

#### Scenario: The banner does not steal focus [up_banner_no_focus]

- **WHEN** the banner appears while a document field has focus
- **THEN** the field keeps focus and the banner is non-activating

#### Scenario: The person chooses Later [up_banner_later]

- **WHEN** the person chooses **Later**
- **THEN** the banner hides for the rest of the launch and the application
  remains usable

### Requirement: Preferences > Updates controls automatic checks

`Edit > Preferences` SHALL include an **Updates** page. The page MUST show an
**Automatically check for updates on launch** checkbox, the running version,
the detected package method, a **Check Now** button, and the last check result.
The checkbox MUST be on by default and MUST persist across restarts.

#### Scenario: The preference defaults on [up_prefs_default_on]

- **WHEN** a fresh session opens Preferences > Updates
- **THEN** the automatic-check checkbox is on

#### Scenario: The preference persists [up_prefs_toggle_persists]

- **WHEN** the person turns automatic checks off and restarts the application
- **THEN** the checkbox is off and no launch request is made

#### Scenario: Check Now runs a manual check [up_prefs_check_now]

- **WHEN** the person presses **Check Now**
- **THEN** the application performs a check and updates the result text

### Requirement: The banner can disable automatic checks

The banner SHALL include a **Don't check for updates automatically** checkbox.
Checking it MUST turn off the automatic launch check, persist the choice, and
update the Preferences checkbox. The person MUST be able to re-enable checks
from Preferences.

#### Scenario: The banner disables automatic checks [up_dont_check_disables]

- **WHEN** the person checks **Don't check for updates automatically**
- **THEN** the session stores automatic checks off and the Preferences
  checkbox follows

#### Scenario: The person re-enables checks [up_reenable]

- **WHEN** the person turns automatic checks on in Preferences
- **THEN** the next launch checks for updates again

### Requirement: Download progress and failures are visible and cancellable

After **Update Now** is chosen, the banner SHALL show download progress and a
**Cancel** action. A failed download MUST show an error with **Retry** and
**Release Page**. The application MUST remain usable throughout.

#### Scenario: A download shows progress [up_progress]

- **WHEN** the update download starts
- **THEN** the banner shows a progress indicator and the received and total
  sizes

#### Scenario: The person cancels a download [up_cancel]

- **WHEN** the person chooses **Cancel** during a download
- **THEN** the download stops, the temporary file is removed, and the banner
  returns to its available-update state

#### Scenario: A download fails [up_retry]

- **WHEN** the download fails
- **THEN** the banner shows the error with **Retry** and **Release Page**

### Requirement: The manual check has a menu entry

The Help menu SHALL contain **Check for Updates…**, and choosing it MUST run
the same manual check as the Preferences button.

#### Scenario: The menu entry checks [up_menu_entry]

- **WHEN** the person chooses `Help > Check for Updates…`
- **THEN** a manual check runs and its result is reported

### Requirement: The version and package method are visible for support

`Help > About` SHALL show the running application version and the detected
package method. The System Info report SHALL include a `Package:` line with the
same method.

#### Scenario: About shows the version and method [up_about_version]

- **WHEN** the person opens `Help > About`
- **THEN** the dialog shows `Kooka Pictura <version>` and the package method

#### Scenario: System Info reports the package method [up_system_info_package]

- **WHEN** the person copies the System Info report
- **THEN** the report contains a `Package:` line with the detected method
