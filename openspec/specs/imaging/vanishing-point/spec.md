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
- **OK with Render Grids To Photoshop on, or after any pixel edit**
  (a dropped selection, a Stamp or Brush stroke) SHALL record one
  "Vanishing Point" state that applies the session's pixels and draws the
  grids, clipped to the selection. A selection still floating at OK SHALL be
  dropped first.
- **OK with neither** SHALL record no state but keep the planes.

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

#### Scenario: OK applies pixel edits as one state [vp_ok_pixels]

- **WHEN** `tst_vanishing_point_retouch::okAppliesAStrokeAsOneState`
  paints a Brush stroke in a plane, with Render Grids off, and presses OK;
  and separately paints and cancels
- **THEN** OK adds one "Vanishing Point" state that changes pixels inside
  the plane only, and Cancel changes nothing

### Requirement: Vanishing Point workspace

The dialog SHALL have:

- a menu button with Show Edges (on, Ctrl+H) and Render Grids To Photoshop
  (off each session)
- Grid Size (1–1000) and Angle (0–360°, enabled only for a plane that may
  turn)
- a help line naming the tool's use, and OK / Cancel
- the tools, in CS6's order, Edit Plane (V), Create Plane (C), Marquee (M),
  Stamp (S), Brush (B), Transform (T), Eyedropper (I), Hand (H) and Zoom
  (Z), with the Extended-only Measure tool absent
- options that follow the tool: Grid Size and Angle for the plane tools;
  Feather, Opacity, Heal and Move Mode for Marquee; Diameter, Hardness,
  Opacity, Heal and Aligned for Stamp; Diameter, Hardness, Opacity, Heal and
  Brush Color for Brush; Flip and Flop for Transform; Brush Color for
  Eyedropper
- the preview, with a zoom row and the title "Vanishing Point (name @
  zoom%)".

It SHALL open in Create Plane when the document has no planes and in Edit
Plane otherwise. Four Create Plane clicks SHALL make a plane and return to
Edit Plane; Backspace SHALL drop the last pending corner and Esc all of them.
Planes SHALL draw in their validity colour, with the active plane's nodes.

#### Scenario: The workspace opens with its controls [vp_controls]

- **WHEN** `tst_vanishing_point::opensWithControls` opens the dialog on a
  document with no planes
- **THEN** the title, the nine tools in order (no Measure), Grid Size 100
  in 1–1000, Angle disabled, Show Edges on, Render Grids off and Create
  Plane selected are as stated

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

- **WHEN** `tst_vanishing_point::toolShortcuts` presses C, M, S, B, T, I,
  H, Z, V
- **THEN** each selects its tool

### Requirement: Perspective editing space

Each plane family SHALL have one flat editing space: a root plane's own
metric coordinates, with every related plane unfolded about its hinge so
that distances from the hinge are kept. Edits SHALL be placed, sized and
moved in this space, and SHALL be mapped into the image through the plane
under each pixel. Edits SHALL change pixels only inside valid planes. A
pixel covered by several planes of a family SHALL take the plane the edit
belongs to first.

#### Scenario: A hinge unfolds into the flat space [vp_flat]

- **WHEN** `vanishing_point::tests_retouch::a_hinge_unfolds_into_the_flat_space`
  maps points on a plane torn off at 90° into the flat space and back
- **THEN** a point a distance t past the hinge lies t past the parent's
  edge in flat space, and image → flat → image returns the point within
  1e-6 px

#### Scenario: Edits stay inside planes [vp_inside]

- **WHEN** `vanishing_point::tests_retouch::edits_stay_inside_planes`
  paints a stroke that runs off a plane and drops a selection larger than it
- **THEN** no pixel outside the plane changes

### Requirement: Floating selections

Paste (Ctrl+V) SHALL bring the clipboard's pixels in as a floating
selection, placed in the active valid plane (or the newest valid one) at one
texel per metric unit, shrunk to fit within 80 % of the plane. With no valid
plane, or an empty clipboard, Paste SHALL be refused with a message. A
floating selection SHALL be drawn through the perspective of the planes it
covers, SHALL follow the pointer when dragged (across hinges and into other
planes), and SHALL be dropped (composited into the layer) when the user
clicks outside it with the Marquee tool, switches to a painting tool, presses
Ctrl+D or presses OK.

Marquee options SHALL be:

- **Feather** (0–250 px): softens the dropped edge.
- **Opacity** (0–100 %, default 100): how much the dropped pixels cover
  what is below.
- **Heal** (Off, Luminance, On; default Off): Off copies the pixels;
  Luminance keeps their colour and takes the destination's lighting; On
  keeps their texture and takes the destination's colour and lighting, with
  no seam.
- **Move Mode** (Destination, Source; default Destination).

The Marquee tool SHALL draw a rectangle in the flat space (Shift: a square),
and a double-click inside a plane SHALL select the whole plane. Dragging
inside a selection SHALL move the marquee (Destination) or fill it from
where the pointer moves to (Source). Alt-drag SHALL float a copy of the
pixels under the selection, and Ctrl-drag SHALL fill it from under the
pointer, whatever the Move Mode.

The Transform tool (Ctrl+T on a floating selection) SHALL show a bounding
box: corner handles scale (Shift keeps the aspect), dragging just outside a
corner rotates, dragging inside moves, and Flip and Flop mirror it.

#### Scenario: Paste places a float in the active plane [vp_paste]

- **WHEN** `vanishing_point::tests_retouch::paste_places_a_float_in_the_active_plane`
  pastes a 40×30 image into a square-on plane, then pastes with no valid
  plane
- **THEN** the float is centred in the plane at one texel per metric unit,
  and the second paste is refused

#### Scenario: A float follows the plane's perspective [vp_float_perspective]

- **WHEN** `vanishing_point::tests_retouch::a_float_follows_the_planes_perspective`
  renders a solid float on a square-on plane and on a tilted one
- **THEN** the square-on footprint covers the texel area within 2 %, and
  on the tilted plane each footprint edge runs through one of the plane's
  vanishing points

#### Scenario: A float wraps across a hinge [vp_float_hinge]

- **WHEN** `vanishing_point::tests_retouch::a_dragged_float_wraps_across_a_hinge`
  drags a float from a root plane onto the plane torn off its edge
- **THEN** the float covers pixels on both planes, and they meet at the
  hinge

#### Scenario: Transform scales, rotates and flips [vp_transform]

- **WHEN** `vanishing_point::tests_retouch::transform_scales_rotates_and_flips`
  scales a float by 2, rotates it 90° and flips it
- **THEN** its footprint grows fourfold, a rotated corner texel lands
  where expected, and the flip mirrors it left to right

#### Scenario: Dropping uses Opacity and Feather [vp_drop]

- **WHEN** `vanishing_point::tests_retouch::dropping_uses_opacity_and_feather`
  drops a white float over black at 50 % opacity, then another with an
  8 px feather
- **THEN** the first covers half-way, and the second is full inside and
  fades over its edge

#### Scenario: Heal modes [vp_heal]

- **WHEN** `vanishing_point::tests_retouch::heal_modes_blend_as_stated`
  drops a flat red float onto flat grey with Heal Off, Luminance and On
- **THEN** Off leaves red, Luminance leaves red's hue at grey's lightness,
  and On leaves grey

#### Scenario: Marquee copies, moves and fills [vp_marquee]

- **WHEN** `vanishing_point::tests_retouch::the_marquee_copies_moves_and_fills`
  draws a marquee and Alt-drags it, then draws another, drags it in
  Destination mode, and Ctrl-drags it
- **THEN** the Alt-drag floats a copy and leaves the source untouched, the
  Destination drag only moves the marquee, and the Ctrl-drag fills the
  marquee from under the pointer

#### Scenario: A double-click selects the whole plane [vp_marquee_plane]

- **WHEN** `vanishing_point::tests_retouch::a_double_click_selects_the_whole_plane`
  double-clicks inside a plane with the Marquee tool
- **THEN** the marquee is the plane's rect

### Requirement: Stamp, Brush and Eyedropper

The Stamp tool SHALL clone within the image in perspective. Alt-click SHALL
set the source point; a stroke SHALL copy from the source offset in the flat
space, so cloned detail scales and foreshortens with the planes. With
Aligned on the offset SHALL stay set from the first stroke; with it off,
each stroke SHALL restart at the source point. The Stamp SHALL NOT clone
from another image.

The Brush tool SHALL paint the Brush Color (the foreground colour when the
dialog opens) in round dabs whose size is in the flat space, so they
foreshorten with the plane. Stamp and Brush SHALL offer:

- **Diameter** (1–5000 px, default 100 *(unverified)*)
- **Hardness** (0–100 %, default 100)
- **Opacity** (0–100 %, default 100)
- **Heal** (Off, Luminance, On)

`[` and `]` SHALL change Diameter. The Eyedropper SHALL set Brush Color
from the preview. Stamp and Brush SHALL paint only inside planes. Every
stroke SHALL be one step of the dialog's undo.

#### Scenario: The Stamp clones in perspective [vp_stamp]

- **WHEN** `vanishing_point::tests_retouch::the_stamp_clones_in_perspective`
  sets a source on a textured half of a square-on plane and paints on the
  other, aligned and then not
- **THEN** the painted pixels repeat the source's texture at the offset,
  and an unaligned second stroke starts again from the source

#### Scenario: Brush dabs foreshorten [vp_brush]

- **WHEN** `vanishing_point::tests_retouch::brush_dabs_foreshorten`
  paints one dab on a square-on plane and one on a plane receding upward
- **THEN** the first footprint is round, and the second is flatter
  vertically than horizontally

#### Scenario: Pixel edits undo and redo [vp_pixel_undo]

- **WHEN** `vanishing_point::tests_retouch::pixel_edits_undo_and_redo`
  paints a stroke, undoes and redoes it
- **THEN** undo restores the layer bit-identically and redo puts the stroke
  back

#### Scenario: The commit replays the session's pixels [vp_patch]

- **WHEN** `vanishing_point::tests_retouch::the_commit_replays_the_sessions_pixels`
  applies a session's edit to the original layer pixels
- **THEN** the result equals the session's working pixels, and an edit
  with grids draws them over the pixels

#### Scenario: The Eyedropper picks from the preview [vp_eyedropper]

- **WHEN** `vanishing_point::tests_retouch::the_eyedropper_picks_from_the_preview`
  samples a painted pixel and a backdrop pixel
- **THEN** each returns the colour the preview shows there

#### Scenario: Paste in the dialog [vp_paste_ui]

- **WHEN** `tst_vanishing_point_retouch::ctrlVPastesIntoThePlane` copies a
  layer, opens the dialog with a plane and presses Ctrl+V; and presses it
  with no plane
- **THEN** a floating selection appears and the Transform tool is
  selectable; with no plane the help line says a plane is needed and
  nothing floats

#### Scenario: A pasted float commits as one state [vp_paste_commit]

- **WHEN** `tst_vanishing_point_retouch::pastedFloatCommitsOnOk` pastes,
  drags the float and presses OK
- **THEN** one state is added and the pasted colour appears inside the
  plane only

#### Scenario: Tool options follow the tool [vp_options]

- **WHEN** `tst_vanishing_point_retouch::optionsFollowTheTool` selects each
  tool
- **THEN** each shows its options, with the stated ranges and defaults

#### Scenario: Stamp and Eyedropper in the dialog [vp_stamp_ui]

- **WHEN** `tst_vanishing_point_retouch::stampAndEyedropperWork` Alt-clicks
  a source, paints with the Stamp, then picks a colour with the Eyedropper
- **THEN** the painted area takes the source's colour, and Brush Color
  becomes the picked colour

#### Scenario: Brush size keys [vp_brush_keys]

- **WHEN** `tst_vanishing_point_retouch::bracketsChangeDiameter` presses
  `]` and `[` with the Brush
- **THEN** Diameter grows, then shrinks back
