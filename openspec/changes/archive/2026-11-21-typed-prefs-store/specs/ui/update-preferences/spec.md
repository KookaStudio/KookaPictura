# Spec Delta

## MODIFIED Requirements

### Requirement: The dialog can disable automatic checks

The dialog SHALL include a **Check for updates automatically** checkbox.
Unchecking it MUST turn off the automatic launch check, persist the choice, and
update the Preferences checkbox. The person MUST be able to re-enable checks
from Preferences.

#### Scenario: The dialog disables automatic checks [up_dont_check_disables]

- **WHEN** the person unchecks **Check for updates automatically**
- **THEN** the preferences store records automatic checks as off and the Preferences
  checkbox follows

#### Scenario: The person re-enables checks [up_reenable]

- **WHEN** the person turns automatic checks on in Preferences
- **THEN** the next launch checks for updates again
