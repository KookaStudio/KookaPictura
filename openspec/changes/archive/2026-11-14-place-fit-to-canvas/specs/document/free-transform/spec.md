# Spec Delta

## MODIFIED Requirements

### Requirement: Channel-less embedded smart-object materialization

`transform_layer` SHALL materialize the raster proxy of a channel-less embedded
smart object from its payload before resampling, when `path` resolves to such a
layer and its payload renders through the existing embedded source path. On
success it SHALL drop the smart object, its preserved `SoLd`/`plLd` blocks, and
its linked-source record so no stale untransformed source can survive the
transform. A channel-less target that is not a decodable embedded object SHALL
be refused with no mutation. A channel-less embedded smart object with a
decodable source is exempt from this materialization rule for a scale or
translation transform: such a session on it SHALL preserve the smart object, its
payload, and its native source dimensions, and SHALL update its transform
non-destructively. A rotation, skew, or warp SHALL use this materialization
path.

#### Scenario: A channel-less embedded object is transformed by the raster op

- **WHEN** `transform_layer` runs on a channel-less embedded smart-object layer with a decodable payload
- **THEN** the layer gains resampled channels, its rect is transformed, and `layer.smart_object` is `None`

#### Scenario: An undecodable channel-less target is refused

- **WHEN** the target has no channels and its embedded payload cannot be rendered
- **THEN** the operation returns false and the document is unchanged


### Requirement: Place starts a Free Transform session

After a successful Place the application SHALL select the newly placed layer and
start a Free Transform session on it from the canvas-fitted rect produced by the
Place fit. For the `File > Place…` command, each successfully placed file SHALL
get its own session, in the order the dialog returns the paths. The next selected
file SHALL be placed only after the current session is committed or ended. The
fit itself is part of the `"Place"` state and MUST NOT add a `"Free Transform"`
state. The fit and a scale or translation committed in the session — and in a
later Free Transform session on the placed layer — SHALL be non-destructive for
the placed smart object: they SHALL preserve the smart object, its payload, and
its native source dimensions, MUST NOT materialize or rasterize it, and a later
scale-up SHALL sample the unchanged original source pixels. A rotation, skew, or
warp SHALL use the existing rasterizing transform path.

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

For a multi-file canvas drop the application SHALL place and fit every supported
file and start one session on the last successfully placed layer; cancelling
that session SHALL leave every placed layer at its fitted rect and SHALL NOT
remove or roll back any `"Place"` history state.

#### Scenario: Menu Place enters the session

- **WHEN** `File > Place…` completes successfully with a supported image
- **THEN** a Free Transform session begins on the new layer from its fitted rect and the `"Place"` state remains

#### Scenario: An oversized placement starts from the fitted rect [ft_place_fit_start]

- **WHEN** an oversized source is placed and the Free Transform session begins
- **THEN** the session's rect is the canvas-fitted rect, and the placed smart object still carries the original source payload and native dimensions

#### Scenario: A source that already fits starts at native scale [ft_place_fit_native]

- **WHEN** a source that does not exceed the canvas is placed and the session begins
- **THEN** the session's rect is the source's native rect and the source is not upscaled

#### Scenario: Scaling a fitted placement up keeps the original source [ft_place_fit_enlarge]

- **WHEN** the user enlarges a fitted placed layer in the Place Free Transform session and commits
- **THEN** one `"Free Transform"` state is added, the layer rect reflects the new scale, the placed smart object and its original payload and native dimensions remain, and the composite is clipped to the canvas

#### Scenario: Canvas drop enters the session

- **WHEN** one or more supported files are dropped on the document canvas
- **THEN** every placed layer is fitted to the canvas, a session begins on the last placed layer from its fitted rect, and every placed layer keeps its `"Place"` state

#### Scenario: Canvas drop fits each oversized layer [ft_canvas_drop_fit]

- **WHEN** one or more oversized supported files are dropped on the canvas
- **THEN** every placed layer is fitted to the canvas and every smart object keeps its original source

#### Scenario: Cancelling keeps the placement

- **WHEN** a session started by a canvas drop is cancelled
- **THEN** the placed layer remains at its fitted rect and the `"Place"` history state is still present

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
