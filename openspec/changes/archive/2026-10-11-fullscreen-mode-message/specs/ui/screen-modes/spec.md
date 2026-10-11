# ui/screen-modes — delta

## ADDED Requirements

### Requirement: Full screen mode advisory message

Entering Full Screen Mode or Full Screen Mode With Menu Bar from Standard Screen
Mode SHALL show the CS6 advisory before the mode change: a modal message box
titled `Message` whose body states that panels are hidden in Full Screen Mode,
that they can be accessed on the sides of the screen or revealed with `Tab`, and
that `F` or `Esc` returns to Standard Screen Mode. The box SHALL offer a default
`Full Screen` button, a `Cancel` button, and a `Don't show again` checkbox.
Cancel SHALL abort the transition and leave the frame in Standard Screen Mode.
Accepting with the checkbox checked SHALL persist the suppression in the session
store, and a suppressed preference SHALL skip the box on later transitions.
Switching between the two full-screen modes SHALL NOT show the box.

#### Scenario: Entering full screen from Standard prompts [smod_advisory]

- **WHEN** the frame is in Standard Screen Mode with the suppression unset and Full Screen Mode is requested
- **THEN** the advisory is shown and accepting it enters Full Screen Mode

#### Scenario: Cancel keeps Standard Screen Mode [smod_advisory_cancel]

- **WHEN** the advisory is shown and Cancel is chosen
- **THEN** the frame remains in Standard Screen Mode

#### Scenario: Don't show again persists [smod_advisory_dont_show]

- **WHEN** the advisory is accepted with `Don't show again` checked, and a full-screen mode is requested again
- **THEN** no advisory is shown and the mode changes directly

#### Scenario: Switching between full-screen modes does not prompt [smod_advisory_skip]

- **WHEN** the frame is already in a full-screen mode and the other full-screen mode is requested
- **THEN** the mode changes without an advisory

### Requirement: Returning to Standard Screen Mode

While in either full-screen mode, `Esc` SHALL return the frame to Standard Screen
Mode once no active tool has claimed the key, and `F` SHALL continue the forward
cycle so that pressing it from Full Screen Mode also reaches Standard Screen
Mode. A tool's cancel handling (a transform session, crop, lasso, or text edit)
SHALL take precedence over the `Esc` return.

#### Scenario: Esc leaves full screen [smod_esc_exit]

- **WHEN** the frame is in a full-screen mode and `Esc` reaches the frame
- **THEN** the frame returns to Standard Screen Mode with the standard chrome

#### Scenario: A tool cancel wins over the mode exit [smod_esc_tool_first]

- **WHEN** a tool holds a cancellable session and `Esc` is pressed in a full-screen mode
- **THEN** the tool cancels and the frame stays in its screen mode

#### Scenario: F returns from full screen [smod_f_return]

- **WHEN** `F` is pressed from Full Screen Mode
- **THEN** the frame returns to Standard Screen Mode

### Requirement: Tab reveals hidden full-screen chrome

In either full-screen mode the panel columns are hidden by the mode; `Tab` SHALL
reveal them on its first press and hide them again on the next. The reveal SHALL
be transient: leaving full screen SHALL restore the Standard-mode panel
visibility that was in effect before the mode change, regardless of how many
times `Tab` toggled the full-screen chrome.

#### Scenario: First Tab reveals the panels [smod_tab_reveal]

- **WHEN** the frame enters a full-screen mode and `Tab` is pressed
- **THEN** the Tools panel and the widget columns are visible

#### Scenario: Second Tab hides them again [smod_tab_hide]

- **WHEN** `Tab` is pressed a second time in the same full-screen mode
- **THEN** the panel columns are hidden again

#### Scenario: The reveal does not change Standard mode [smod_tab_transient]

- **WHEN** panels are revealed with `Tab` in a full-screen mode and the frame returns to Standard Screen Mode
- **THEN** the panel columns show exactly as they did before the full-screen transition
