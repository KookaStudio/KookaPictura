# Spec Delta

## MODIFIED Requirements

### Requirement: Place starts a Free Transform session

After a successful Place the application SHALL select the newly placed layer and
start a Free Transform session on it. For the `File > Place…` command, each
successfully placed file SHALL get its own session, in the order the dialog
returns the paths. The next selected file SHALL be placed only after the current
session is committed or ended.

For a menu Place selection, the Free Transform session SHALL still cancel to its
pre-session state as defined by the core session requirement. Committing a
changed transform with Enter or the apply control SHALL record exactly one
`"Free Transform"` history state and continue with the next file. Committing an
identity transform SHALL record no history state and SHALL also continue with
the next file. Cancelling with Escape or the transform's Cancel control
(`optionsTransformCancel`) SHALL make the Place command remove the just-placed
layer and roll back its `"Place"` history state as a separate step. A session
that ends any other way — for example a tool switch choosing Don't Apply, or any
non-interruption route that leaves the `"Place"` state — SHALL keep the placed
layer and its `"Place"` state and continue with the next file. If an undo
already removed the `"Place"` state, the command SHALL treat the item as
cancelled without a second undo. If the rollback fails, the command SHALL
surface the existing memory refusal, SHALL NOT claim the placement was
cancelled, and SHALL stop the batch without the cancel/continue prompt. When
more selected files remain after an Escape/Cancel cancellation, the application
SHALL ask whether to cancel the whole Place or continue with the next item;
closing that prompt SHALL cancel the whole Place. Escape or Cancel on the last
item SHALL end the Place without the prompt. A profile-warning cancellation
SHALL skip only that file and SHALL NOT be treated as a transform cancellation.

Another document becoming active SHALL abort the batch; the original document
SHALL keep its placed layers and the application SHALL NOT show the failure
summary. Closing the document SHALL abort the batch without showing the failure
summary.

For a multi-file canvas drop the behaviour is unchanged: the application SHALL
place every supported file and start one session on the last successfully placed
layer; cancelling that session SHALL leave every placed layer where it landed
and SHALL NOT remove or roll back any `"Place"` history state.

#### Scenario: Menu Place enters the session

- **WHEN** `File > Place…` completes successfully with a supported image
- **THEN** a Free Transform session begins on the new layer and the `"Place"` state remains

#### Scenario: Canvas drop enters the session

- **WHEN** one or more supported files are dropped on the document canvas
- **THEN** a session begins on the last placed layer and every placed layer keeps its `"Place"` state

#### Scenario: Cancelling keeps the placement

- **WHEN** a session started by a canvas drop is cancelled
- **THEN** the placed layer remains at its landed rect and the `"Place"` history state is still present

#### Scenario: Menu Place sequences one session per file [ft_menu_multi_place]

- **WHEN** two files are chosen in `File > Place…`
- **THEN** the first layer enters Free Transform before the second file is placed, and committing it with Enter or the apply control places the second file and starts its session

#### Scenario: Escape or Cancel cancels the current placement [ft_menu_place_escape]

- **WHEN** the user presses Escape or uses the transform's Cancel control during a menu Place Free Transform session
- **THEN** the just-placed layer and its `"Place"` history state are removed and the document matches its pre-place state

#### Scenario: Escape or Cancel asks before the next item [ft_menu_place_escape_next]

- **WHEN** the user presses Escape or uses the Cancel control on an item and more selected files remain
- **THEN** one dialog offers to cancel the whole Place or continue with the next item, and continuing places the next file

#### Scenario: Escape or Cancel on the last item ends the Place [ft_menu_place_escape_last]

- **WHEN** the user presses Escape or uses the Cancel control on the last selected file
- **THEN** the placement is removed and the Place ends without the cancel/continue dialog

#### Scenario: Cancelling the whole Place stops the batch [ft_menu_place_escape_whole]

- **WHEN** the user chooses to cancel the whole Place after Escape or the Cancel control
- **THEN** no further files are placed and previously committed items remain in the document

#### Scenario: A tool switch that does not apply keeps the placement [ft_tool_switch_dont_apply]

- **WHEN** the user chooses another tool during a menu Place Free Transform session and answers Don't Apply
- **THEN** the placed layer and its `"Place"` state remain and the next selected file is placed

#### Scenario: An identity commit advances without a history state [ft_menu_place_identity_commit]

- **WHEN** the user commits a menu Place Free Transform session without changing the transform
- **THEN** no `"Free Transform"` history state is added and the next selected file is placed

#### Scenario: Another document becoming active aborts the batch [ft_menu_place_document_switch]

- **WHEN** another document becomes active while the menu Place Free Transform session is pending
- **THEN** the batch stops, the original document keeps its placed layers, and no failure summary is shown

#### Scenario: Closing the document aborts the batch [ft_menu_place_document_close]

- **WHEN** the document is closed while the menu Place Free Transform session is pending
- **THEN** the batch stops and no failure summary is shown
