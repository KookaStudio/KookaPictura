# Workspace shell polish delta

## MODIFIED Requirements

### Requirement: Paint Mask Mode toolbar button

The Tools panel SHALL show a working Paint Mask Mode toggle button in the slot
immediately to the left of the screen-mode button, retaining the existing
one-column stacked arrangement and shared slot metrics. Its checked state SHALL
reflect the active document's Quick Mask mode. A click or `Q` SHALL toggle that
mode through the same command, and the control SHALL be disabled without a
document. Double-click SHALL open Quick Mask Options without unintentionally
entering or leaving the mode; `Alt`-click SHALL change display polarity without
toggling mode. Text entry and modal dialogs SHALL retain their normal ownership
of keyboard input.

#### Scenario: The button sits left of screen mode and toggles [lwsp_paint_mask_stub]
- **WHEN** a document is open and the Paint Mask button is clicked and then `Q` is pressed
- **THEN** the button retains its footer position, Quick Mask enters and leaves, and the button tracks the mode rather than an independent checked state

#### Scenario: The button follows document availability [lwsp_paint_mask_document]
- **WHEN** the active document changes or the last document closes
- **THEN** the button shows the active document's mode and is disabled when no document remains

#### Scenario: Options gestures do not toggle the mode [lwsp_paint_mask_gestures]
- **WHEN** the button is double-clicked or `Alt`-clicked in either Standard or Quick Mask mode
- **THEN** the options dialog opens or polarity changes respectively, without changing whether Quick Mask is active

#### Scenario: Typing Q does not toggle Quick Mask [lwsp_paint_mask_text]
- **WHEN** `Q` is typed into a text editor or an owning modal dialog
- **THEN** normal text or dialog input occurs and Quick Mask mode is unchanged
