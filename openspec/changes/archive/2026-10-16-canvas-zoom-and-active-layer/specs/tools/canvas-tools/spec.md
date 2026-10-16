## MODIFIED Requirements

### Requirement: Hand and Zoom tools

The system SHALL provide Hand and Zoom as first-class tools. Hand SHALL pan the
canvas on drag; Zoom SHALL magnify on click and reduce on modified click. The
canvas offset SHALL be clamped by one shared range helper at every mutation point
— pan, zoom anchoring, centre, fit, actual pixels, initial view, resize, and the
Navigator proxy — so that at least one display inch of 96 logical pixels of the
canvas remains visible on each axis whenever the document is larger than that
axis. The clamp SHALL slide each axis by the minimum needed rather than
recentering, and the canvas SHALL remain the single source of the offset.
**Zoom-in SHALL target the pointer: the Zoom tool's zoom-in click SHALL anchor
the zoom at the clicked image point, and a wheel zoom-in SHALL keep the point
under the cursor fixed. Zoom-out SHALL anchor at the canvas centre so the image
does not drift off the pointer: a wheel zoom-out, an `Alt`/`Option` (modified)
Zoom-tool click, and `View > Zoom Out` SHALL all step about the viewport centre.
Dragging with the Zoom tool (press, move, release) SHALL draw a marquee, and on
release the view SHALL zoom so that rectangle fills the viewport at the highest
magnification that fits it, centred on the rectangle; holding `Space` mid-marquee
SHALL reposition the marquee instead of resizing it. A press-release with no drag
SHALL keep the click step. The Navigator slider SHALL anchor at the click/cursor
point on the proxy. The wheel SHALL apply one shared modifier precedence: a
horizontal side-wheel (`angleDelta().x() != 0`) SHALL pan horizontally; a
`Ctrl+Alt` vertical wheel SHALL pan vertically; an `Alt` (without `Ctrl`)
vertical wheel SHALL pan horizontally; otherwise the vertical wheel SHALL zoom,
with `Shift` doubling the wheel zoom step.**

#### Scenario: Zoom in by click

- **WHEN** the Zoom tool is clicked on the canvas
- **THEN** the canvas magnification increases

#### Scenario: Zoom out by modified click

- **WHEN** the Zoom tool is clicked while a modifier is held
- **THEN** the canvas magnification decreases

#### Scenario: Zoom click targets the cursor [lct_zoom_cursor]

- **WHEN** the Zoom tool clicks a point away from the canvas centre without a modifier
- **THEN** the zoom is anchored at that point so it stays under the cursor rather
  than the view zooming about the centre

#### Scenario: Zoom-out anchors at the canvas centre [zta_zoom_out_centre]

- **WHEN** the wheel zooms out, or the Zoom tool is clicked with `Alt`/`Option`
- **THEN** the document centre stays at (near) the centre of the viewport and the
  image does not drift toward the pointer

#### Scenario: A Zoom drag draws a marquee [zta_marquee]

- **WHEN** the Zoom tool is pressed, dragged over a sub-rectangle, and released
- **THEN** a marquee is drawn during the drag, and on release the view zooms so
  that rectangle fills the viewport at the highest magnification that fits it,
  centred

#### Scenario: A click without a drag still steps once [zta_click_steps]

- **WHEN** the Zoom tool is pressed and released at the same point
- **THEN** the zoom steps once (in, or out with `Alt`/`Option`) rather than
  drawing a marquee

#### Scenario: Shift doubles the wheel step [lct_wheel_shift]

- **WHEN** the vertical wheel is scrolled with Shift held
- **THEN** the zoom step is twice the unmodified step, still anchored the same way
  (cursor for zoom-in, centre for zoom-out)

#### Scenario: Alt pans horizontally [lct_wheel_alt_pan]

- **WHEN** the vertical wheel is scrolled with Alt held (without Ctrl)
- **THEN** the canvas pans horizontally and does not zoom

#### Scenario: Ctrl+Alt pans vertically [lct_wheel_ctrl_alt_pan]

- **WHEN** the vertical wheel is scrolled with Ctrl+Alt held
- **THEN** the canvas pans vertically and does not zoom

#### Scenario: A side-wheel pans horizontally [lct_wheel_side_pan]

- **WHEN** a horizontal side-wheel event arrives
- **THEN** the canvas pans horizontally and does not zoom

#### Scenario: Panning cannot push the canvas fully off-screen [lct_pan_margin]

- **WHEN** the canvas is panned in any direction past the reveal margin
- **THEN** the offset is clamped so at least 96 logical pixels of the canvas stay
  visible on that axis and the canvas does not jump to the other side

#### Scenario: Zoom anchoring is clamped [lct_zoom_clamp]

- **WHEN** a zoom-out at a corner would place the canvas outside the reveal margin
- **THEN** the resulting offset is clamped by the same shared range helper

#### Scenario: Fit and centre remain fully visible [lct_fit_within]

- **WHEN** Fit on Screen or centre is applied
- **THEN** the resulting offset is inside the shared range and the canvas is not
  pushed off-screen

## ADDED Requirements

### Requirement: Zoom tool context menu

Right-clicking the Zoom tool's toolbox slot SHALL open a menu with **Fit on
Screen**, **100%**, **200%**, **Print Size**, a separator, **Zoom In**, and
**Zoom Out**. Each entry SHALL apply to the active document's canvas and SHALL be
disabled when no document is open. Fit on Screen, 100%, Zoom In, and Zoom Out
SHALL match the View menu commands; 200% SHALL set the zoom to 200%; Print Size
SHALL set the zoom so the document displays at its print size, one image inch per
logical screen inch, from the document's resolution (CS6's 72 ppi default). The
Zoom slot SHALL show no member flyout triangle.

#### Scenario: The Zoom slot opens the preset menu [ztm_menu]

- **WHEN** the Zoom tool's toolbox slot is right-clicked
- **THEN** a menu with Fit on Screen, 100%, 200%, Print Size, Zoom In, and Zoom
  Out opens, and the slot shows no member flyout triangle

#### Scenario: A preset applies to the canvas [ztm_apply]

- **WHEN** 200%, 100%, Print Size, or Fit on Screen is chosen with a document open
- **THEN** the active canvas zooms to 200%, 100%, the print-size zoom
  (`96 / ppi`), or the fit zoom respectively
