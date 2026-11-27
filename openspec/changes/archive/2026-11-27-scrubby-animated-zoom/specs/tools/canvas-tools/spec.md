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
With Scrubby Zoom off, dragging with the Zoom tool (press, move, release) SHALL draw a marquee, and on
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

- **WHEN** Scrubby Zoom is off and the Zoom tool is pressed, dragged over a sub-rectangle, and released
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


### Requirement: Scrubby and Animated Zoom controls

The Zoom options bar SHALL offer Scrubby Zoom, initially on. When on, horizontal drag SHALL continuously change magnification logarithmically, rightward in and leftward out, anchored at the press point and subject to the canvas zoom limits; reversal at a limit SHALL respond immediately. A click without a drag SHALL retain the existing discrete step. The change SHALL remain view state and SHALL respect Zoom All Windows, Shift, and Resize Windows To Fit. Cancelling, deactivating the tool, or losing or replacing the document or view SHALL end the gesture without later updates.

General preferences SHALL offer a persisted Animated Zoom preference, initially off, when the active canvas supports timed zoom updates. The current raster canvas SHALL support this capability; GPU compute availability SHALL NOT determine it. When enabled and supported, holding the Zoom tool stationary past the hold delay SHALL continuously zoom until released, in at the press point or out about the viewport centre with the zoom-out modifier. A short click SHALL retain its discrete step; release after animation SHALL NOT add another step. A drag before animation SHALL enter the applicable scrub or marquee gesture. Animation SHALL respect zoom limits, Zoom All Windows, Shift, Resize Windows To Fit, and view-only semantics. Release, a conflicting zoom or pan, cancellation, tool deactivation, document/view loss or replacement, or reaching a limit SHALL stop animation without late updates. If the preference is off or the capability unavailable, a hold SHALL use the discrete click fallback; the unavailable control SHALL explain why it is disabled.

#### Scenario: Scrub in and out [scrub_direction]

- **WHEN** Scrubby Zoom is on and the user drags right and then left
- **THEN** the zoom increases and then decreases continuously about the press point

#### Scenario: Scrub clamps and ends [scrub_limits_cancel]

- **WHEN** a scrub reaches either zoom limit and reverses direction, or the gesture is cancelled, deactivated, or loses its document or view
- **THEN** zoom remains within limits, reversal responds immediately, and no further gesture updates affect the old or replacement view after termination

#### Scenario: Scrub fans out [scrub_multiwindow]

- **WHEN** the user scrubs with Zoom All Windows or Shift enabled
- **THEN** the other document windows follow the zoom factor and Resize Windows To Fit remains effective when selected

#### Scenario: Scrub leaves history unchanged [scrub_history]

- **WHEN** the user completes a scrub
- **THEN** document pixels and edit history remain unchanged

#### Scenario: Animated hold zoom [animated_hold]

- **WHEN** Animated Zoom is enabled on the supported raster canvas and the user holds the Zoom tool past the delay, in either direction
- **THEN** timed updates change the zoom continuously with the appropriate anchor and fanout, respect limits and view-only state, and release adds no extra step

#### Scenario: Animated Zoom fallback [animated_fallback]

- **WHEN** Animated Zoom is off or the canvas capability is unavailable, including when GPU compute is available
- **THEN** a hold gives one discrete step without timed updates, and the unavailable preference is disabled with an explanation

#### Scenario: Animated Zoom interruption [animated_interrupt]

- **WHEN** the user releases, cancels, switches tools or documents, loses the view, or initiates another zoom or pan during an animated hold
- **THEN** the animation stops without a late update to the old or replacement view

#### Scenario: Drag before hold [animated_drag]

- **WHEN** a press moves past the drag threshold before the hold delay expires
- **THEN** the selected scrub or marquee gesture takes over without a timed zoom or extra click step
