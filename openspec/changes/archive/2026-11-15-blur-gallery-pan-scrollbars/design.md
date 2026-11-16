# Design

## Scrollbars own the pan

The canvas keeps the preview as one widget and adds two child `QScrollBar`s at
the right and bottom edges, inside the 24 px pasteboard margin. The picture's
room stays the widget minus the margins whether or not a scrollbar shows, so
the fit zoom and the pin geometry do not change as the scrollbars appear.

`updateScrollBars` ranges each bar over `ceil(picture − room)`, sets its page
and single steps to the room, and shows it only when that range is positive.
`imageRect` centres an axis that fits and subtracts the bar's value from the
room's edge on an axis that overflows.

## Zoom and pan

`zoomAbout` solves the scroll value that puts the document fraction under the
wheel anchor back under it; a fitted axis stays centred. The middle button
records both scroll values and the press point, and each move sets the values
from the drag delta, so the picture follows the pointer. The canvas restores
the default cursor on release.

## No commit impact

The scrollbars and pan only place the preview. Pin fractions and blur amounts,
and so the values OK commits, are unchanged.
