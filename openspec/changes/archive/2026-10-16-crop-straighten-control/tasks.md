# Tasks

## 1. Control

- [x] 1.1 The Straighten toggle shows its label beside the icon (`ToolButtonTextBesideIcon`); Qt Test asserts the label and style.
- [x] 1.2 Holding Ctrl arms the line through `onModifiers`; a Ctrl press draws a line; Qt Test covers hold/release and the toggle-armed case.
- [x] 1.3 The toggle's icon is a spirit level (`crop.straighten`, qrc, `assets/PROVENANCE.md`); Qt Test asserts it renders.

## 2. Angle and fit

- [x] 2.1 `straightenAngle` folds to the nearer axis; the line adds to the session angle; Qt Test over level, plumb, reversed, and degenerate lines.
- [x] 2.2 `fitBoxInRotated` shrinks the box on release; Qt Test over the fit geometry and an end-to-end line at 5° (angle within 0.1°, no uncovered corners, commit size).

## 3. Feedback

- [x] 3.1 The line drag shows the `∠` readout of the coming angle, cleared on release; the image rotates only on release; Qt Test asserts no rotation mid-drag and the canvas angle after.
- [x] 3.2 `cursor.straighten` (qrc, `assets/PROVENANCE.md`) while armed; Qt Test asserts the cursor kind.

## 4. Test plan

- [x] 4.1 `test-plan.md` maps every scenario to a test or a justified manual check.
- [x] 4.2 Qt Tests for the remaining gaps: a Ctrl press with no key event draws, releasing Ctrl mid-line finishes it, and a second line adds to the angle.
