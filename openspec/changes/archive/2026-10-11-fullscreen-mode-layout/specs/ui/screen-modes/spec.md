# ui/screen-modes — delta

## MODIFIED Requirements

### Requirement: Tab reveals hidden full-screen chrome

Full Screen Mode With Menu Bar SHALL keep the Tools panel, the Options bar, and
the panels visible as Standard Screen Mode does, hiding only the document-window
chrome (tab, scroll bars, status bar, and window buttons); `Tab` and `Shift+Tab`
there SHALL toggle the same persisted panel visibility as in Standard Screen
Mode. Full Screen Mode SHALL hide the panel columns and the menu bar; `Tab` SHALL
reveal them on its first press and hide them again on the next. That reveal
SHALL be transient: leaving Full Screen Mode SHALL restore the Standard-mode
chrome that was in effect before the mode change, regardless of how many times
`Tab` toggled it.

#### Scenario: Full Screen With Menu Bar keeps the panels [smod_tab_menu_panels]

- **WHEN** the frame enters Full Screen Mode With Menu Bar from Standard Screen Mode
- **THEN** the menu bar, the Tools panel, and the widget columns are visible

#### Scenario: Tab toggles the panels in Full Screen With Menu Bar [smod_tab_menu_toggle]

- **WHEN** the frame is in Full Screen Mode With Menu Bar and `Tab` is pressed
- **THEN** the panel columns hide, and a second `Tab` shows them again

#### Scenario: First Tab reveals the panels [smod_tab_reveal]

- **WHEN** the frame enters Full Screen Mode and `Tab` is pressed
- **THEN** the Tools panel, the widget columns, and the menu bar are visible

#### Scenario: Second Tab hides them again [smod_tab_hide]

- **WHEN** `Tab` is pressed a second time in Full Screen Mode
- **THEN** the panel columns and the menu bar are hidden again

#### Scenario: The reveal does not change Standard mode [smod_tab_transient]

- **WHEN** the chrome is revealed with `Tab` in Full Screen Mode and the frame returns to Standard Screen Mode
- **THEN** the menu bar and the panel columns show exactly as they did before the full-screen transition
