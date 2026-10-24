# imaging/vanishing-point Specification

## Purpose
`Filter ▸ Vanishing Point…`: perspective planes, their validity and
relations, the editing gestures, Render Grids, the commit, and the
workspace.

## Requirements

### Requirement: Perspective plane validity

The system SHALL define a root perspective plane by four image corners. The
principal point SHALL be the document centre. The plane SHALL read as one of
three states:

- **Red** when the quad is crossed, concave or degenerate.
- **Yellow** when it is convex but its two vanishing points admit no real
  focal length (`f² ≤ 0`).
- **Blue (valid)** otherwise.

A vanishing point farther than 20 half-diagonals from the centre SHALL count
as at infinity, and a normal lens SHALL then be assumed. A valid plane SHALL
show a grid at multiples of Grid Size (default 100) in a metric whose area
equals the plane's image area.

#### Scenario: A square-on rectangle is valid with an even grid [vp_square_on]

- **WHEN** `vanishing_point::tests::a_square_on_rectangle_is_valid_with_an_even_grid`
  adds a 400 px square centred in an 800×600 document
- **THEN** the plane is valid and its grid has three inner lines each way,
  100 px apart

#### Scenario: Crossed, concave and degenerate quads are red [vp_red]

- **WHEN** `vanishing_point::tests::crossed_concave_and_degenerate_quads_are_red`
  asks for the validity of a crossed, a concave and a collinear quad
- **THEN** each reads red (no aspect ratio)

#### Scenario: A quad no camera sees as a rectangle is yellow [vp_yellow]

- **WHEN** `vanishing_point::tests::a_quad_no_camera_sees_as_a_rectangle_is_yellow`
  adds a convex quad whose vanishing points both lie up and to the right of
  the centre
- **THEN** the plane reads yellow and draws no grid

#### Scenario: A projected rectangle recovers its camera [vp_camera]

- **WHEN** `vanishing_point::tests::a_projected_rectangle_recovers_its_camera_and_grid_converges`
  adds the image of a tilted 3D rectangle seen through a known camera
- **THEN** the recovered focal length matches the camera's within 1e-6,
  and every grid line runs through one of the two vanishing points

### Requirement: Plane editing

The system SHALL let the dialog edit planes:

- A root corner SHALL move freely, re-fitting the plane.
- An edge node SHALL resize the plane in perspective, leaving its grid lines
  where they are on the surface.
- Dragging the inside SHALL move the plane.

The pointer SHALL pick nodes before planes: the active plane's first, then
the newest plane's. Ctrl-click SHALL cycle through the planes under the
pointer. Deleting a plane SHALL delete the planes hinged on it. The dialog's
undo and redo SHALL step through plane edits, a whole drag being one step.

#### Scenario: Resizing an edge leaves the grid on the surface [vp_edge_resize]

- **WHEN** `vanishing_point::tests::resizing_an_edge_leaves_the_grid_on_the_surface`
  drags a tilted plane's edge node outward
- **THEN** the plane stays valid, gains grid lines, and every earlier line
  is still there, unchanged or only lengthened along itself

#### Scenario: Hits prefer nodes, then the newest plane [vp_hits]

- **WHEN** `vanishing_point::tests::hits_prefer_nodes_then_the_newest_plane`
  hit-tests two overlapping planes with the first active
- **THEN** a corner and an edge node of the active plane are found, the
  overlap picks the active plane, an area only the other plane covers picks
  it, and empty ground picks nothing

#### Scenario: Cycling, delete, undo and redo [vp_editor]

- **WHEN** `vanishing_point::tests::the_editor_creates_cycles_deletes_and_undoes`
  creates two overlapping planes, cycles twice at the overlap, deletes, then
  undoes and redoes
- **THEN** the cycle alternates between the planes, delete removes the
  active one, and undo and redo step back and forth until nothing is left
  to undo

### Requirement: Related planes

The system SHALL tear off a related plane by Ctrl-dragging a valid plane's
edge node:

- The new plane SHALL hinge on that edge at 90° and span the edge.
- Its far edge SHALL follow the pointer, on the side of the parent the
  pointer is on.

A related plane's angle SHALL be set by the Angle box or by Alt-dragging its
far edge node. 0° lies flat beyond the edge. A root, and a related plane that
has related planes of its own, SHALL NOT turn.

Related planes SHALL follow their parent's edits. Their grid lines SHALL
continue the parent's across the hinge. Moving a related plane SHALL slide
it along its hinge. When a family's root is invalid, every plane in it SHALL
take that colour and show its last outline.

#### Scenario: A torn-off plane is perpendicular and shares the edge [vp_tear_off]

- **WHEN** `vanishing_point::tests::a_torn_off_plane_is_perpendicular_and_shares_the_edge`
  tears a plane off edge 1 of a tilted plane and drags it out
- **THEN** it is valid, at 90°, its normal is perpendicular to its parent's,
  and its first two corners are the parent's corners 1 and 2

#### Scenario: A tear-off drag is one undo step [vp_tear_off_undo]

- **WHEN** `vanishing_point::tests::a_tear_off_drag_is_one_undo_step`
  Ctrl-drags an edge node through several moves, then undoes; and tries an
  Alt-drag on a root and a tear-off from a red plane
- **THEN** the drag made the new plane active, one undo removes it, and
  the other two gestures are refused

#### Scenario: The angle turns a related plane about its hinge [vp_angle]

- **WHEN** `vanishing_point::tests::the_angle_turns_a_related_plane_about_its_hinge`
  sets 45° and 0°, then aims the far edge node at where it sits at 60°
- **THEN** the planes' normals meet at 45°, then coincide, and the aim
  recovers 60° within 0.1°

#### Scenario: A plane with related planes cannot turn [vp_angle_lock]

- **WHEN** `vanishing_point::tests::a_plane_with_related_planes_cannot_turn`
  tears a grandchild off a related plane
- **THEN** neither the root nor the middle plane may turn, and the
  grandchild may

#### Scenario: Related planes follow their parent [vp_follow]

- **WHEN** `vanishing_point::tests::related_planes_follow_their_parent`
  moves the parent's corner, then crosses the parent
- **THEN** the related plane stays hinged on the parent's edge, and once
  the parent is red the related plane is red with its outline frozen

#### Scenario: A related plane slides along its hinge [vp_slide]

- **WHEN** `vanishing_point::tests::a_slid_related_plane_stays_on_its_hinge`
  drags a related plane's inside
- **THEN** it moves and its hinge corners stay on the parent's edge line

#### Scenario: Grid lines continue across a flat hinge [vp_grid_continues]

- **WHEN** `vanishing_point::tests::grid_lines_continue_across_a_flat_hinge`
  turns a related plane to 0°
- **THEN** every parent grid line ending on the hinge has a related-plane
  line starting at the same point

#### Scenario: Removing a plane removes its related planes [vp_remove]

- **WHEN** `vanishing_point::tests::removing_a_plane_removes_its_related_planes`
  removes the first of two families
- **THEN** the other family stays intact, re-indexed

### Requirement: Render Grids

The system SHALL implement `Filter::VanishingPoint` over a kept edit. The
edit SHALL hold every valid plane's outline and grid lines, in the active
layer's pixels. The kernel SHALL draw them one pixel wide and antialiased in
black over the layer, adding opacity on transparent pixels. An empty edit
SHALL change nothing. The result SHALL be deterministic. The
`vanishing-point` kind's slot SHALL name a kept edit by id, where 0 draws
nothing, and an id no longer kept SHALL be refused.

#### Scenario: Render Grids draws black lines into the layer [vp_render]

- **WHEN** `vanishing_point::tests::render_grids_draws_black_lines_into_the_layer`
  renders a square-on plane's grid into an offset opaque layer and a clear
  one
- **THEN** pixels on grid lines and the outline darken, cells and the
  ground outside stay unchanged, and on the clear layer the lines are black
  and partly opaque

#### Scenario: An empty edit changes nothing [vp_render_empty]

- **WHEN** `vanishing_point::tests::an_empty_edit_changes_nothing_and_runs_are_deterministic`
  applies an empty edit, then one edit twice
- **THEN** the empty edit leaves the buffer bit-identical and the two
  runs are byte-identical

#### Scenario: The slot names a kept edit [vp_kept_edit]

- **WHEN** `filter_map_tests::vanishing_point_names_a_kept_edit_and_refuses_one_it_no_longer_has`
  and `vanishing_point::tests::kept_edits_are_found_by_id_and_zero_draws_nothing` map
  ids
- **THEN** id 0 and no slots map to an empty edit, a kept id maps to its
  lines, and an evicted id, an unknown id and a wrong arity are refused

### Requirement: Vanishing Point commit

`Filter ▸ Vanishing Point…` (Alt+Ctrl+V) SHALL open the dialog over the
active pixel layer of an 8-bit RGB or Grayscale document. Elsewhere the menu
row SHALL be disabled, and every commit path SHALL refuse. The commit
outcomes SHALL be:

- **Cancel** SHALL change nothing.
- **OK with Render Grids To Photoshop on** SHALL record one "Vanishing
  Point" state that draws the grids, clipped to the selection.
- **OK without Render Grids** SHALL record no state but keep the planes.

The document SHALL keep its planes, so the dialog reopens on them; a new
document SHALL start with none. Last Filter Settings SHALL reopen on the
committed edit's planes with Render Grids on.

#### Scenario: The menu row follows the document [vp_menu]

- **WHEN** `tst_vanishing_point::menuFollowsDepth` checks the row on an
  8-bit document and after Image ▸ Mode ▸ 16 Bits/Channel
- **THEN** the row has Alt+Ctrl+V and one slot, and is enabled, then
  disabled

#### Scenario: Cancel changes nothing [vp_cancel]

- **WHEN** `tst_vanishing_point::cancelChangesNothing` draws a plane, turns
  on Render Grids and cancels
- **THEN** the image and the history are unchanged and reopening shows no
  planes

#### Scenario: OK renders the grids as one state [vp_ok_render]

- **WHEN** `tst_vanishing_point::okRendersGridsAsOneState` draws a plane,
  turns on Render Grids and presses OK
- **THEN** one "Vanishing Point" state is added, the image changes inside
  the plane and nowhere outside it, and Last Filter is `vanishing-point`

#### Scenario: OK without Render Grids keeps only the planes [vp_ok_planes]

- **WHEN** `tst_vanishing_point::okWithoutRenderKeepsPlanesOnly` draws a
  plane and presses OK with Render Grids off, then reopens the dialog
- **THEN** no state is added, the image is unchanged, the dialog reopens
  with the plane, Render Grids is off again, and a new document opens with
  no planes

#### Scenario: The render is clipped to the selection [vp_selection]

- **WHEN** `tst_vanishing_point::renderIsClippedToTheSelection` renders a
  plane's grid with a selection over half of it
- **THEN** pixels change inside the selection only

#### Scenario: Last Filter Settings reopens on the planes [vp_last_settings]

- **WHEN** `tst_vanishing_point::lastSettingsRestoresPlanes` commits two
  planes with Render Grids, deletes them in a later session, then runs
  Last Filter Settings
- **THEN** the dialog opens with the two planes and Render Grids on

#### Scenario: Off 8-bit every path refuses [vp_last_depth]

- **WHEN** `tst_vanishing_point::lastFilterRefusesOffEightBit` commits,
  converts to 16 Bits/Channel and runs Last Filter Settings and Last Filter,
  and `filter_tools::vanishing_point::tests::commits_are_refused_off_eight_bit_rgb_and_gray`
  asks every mode
- **THEN** no dialog opens, no state is added, the refusal names 8-bit,
  and only 8-bit RGB and Grayscale pass

### Requirement: Vanishing Point workspace

The dialog SHALL have:

- a menu button with Show Edges (on, Ctrl+H) and Render Grids To Photoshop
  (off each session)
- Grid Size (1–1000) and Angle (0–360°, enabled only for a plane that may
  turn)
- a help line naming the tool's use, and OK / Cancel
- the tools Edit Plane (V), Create Plane (C), Hand (H) and Zoom (Z), with
  the Extended-only Measure tool absent
- the preview, with a zoom row and the title "Vanishing Point (name @
  zoom%)".

It SHALL open in Create Plane when the document has no planes and in Edit
Plane otherwise. Four Create Plane clicks SHALL make a plane and return to
Edit Plane; Backspace SHALL drop the last pending corner and Esc all of them.
Planes SHALL draw in their validity colour, with the active plane's nodes.

#### Scenario: The workspace opens with its controls [vp_controls]

- **WHEN** `tst_vanishing_point::opensWithControls` opens the dialog on a
  document with no planes
- **THEN** the title, the four tools (no Measure), Grid Size 100 in 1–1000,
  Angle disabled, Show Edges on, Render Grids off and Create Plane selected
  are as stated

#### Scenario: Four clicks create a plane [vp_create]

- **WHEN** `tst_vanishing_point::fourClicksCreateAPlane` clicks three
  corners, presses Backspace, clicks two more, and separately presses Esc
  after two
- **THEN** one valid plane exists, Edit Plane is selected, and Esc left no
  pending corner

#### Scenario: Dragging reshapes and moves [vp_drag]

- **WHEN** `tst_vanishing_point::dragReshapesAndMoves` drags a corner and
  then the inside of a plane
- **THEN** the corner follows the pointer, and the move shifts every
  corner by the drag

#### Scenario: Ctrl-drag tears off and Angle turns [vp_tear_off_ui]

- **WHEN** `tst_vanishing_point::ctrlDragTearsOffAndAngleTurns` Ctrl-drags
  an edge node, sets Angle to 45, then selects the parent
- **THEN** a second plane is active with Angle enabled at 90, then 45; with
  the parent active Angle is disabled

#### Scenario: Ctrl-click cycles and Delete removes [vp_cycle_ui]

- **WHEN** `tst_vanishing_point::ctrlClickCyclesAndDeleteRemoves`
  Ctrl-clicks an overlap of two planes, then presses Delete
- **THEN** the active plane changes, and Delete removes it

#### Scenario: Show Edges hides the overlays [vp_show_edges]

- **WHEN** `tst_vanishing_point::showEdgesHidesOverlays` presses Ctrl+H
  twice
- **THEN** the canvas hides, then shows, the planes

#### Scenario: Undo and redo plane edits [vp_undo_ui]

- **WHEN** `tst_vanishing_point::undoRedoPlaneEdits` makes a plane and
  presses Ctrl+Z, then Ctrl+Shift+Z
- **THEN** the plane goes, then comes back

#### Scenario: Grid Size changes the grid [vp_grid_size]

- **WHEN** `tst_vanishing_point::gridSizeChangesTheGrid` sets Grid Size from
  100 to 50
- **THEN** the active plane draws more grid lines

#### Scenario: Tool shortcuts [vp_tool_keys]

- **WHEN** `tst_vanishing_point::toolShortcuts` presses C, H, Z, V
- **THEN** each selects its tool
