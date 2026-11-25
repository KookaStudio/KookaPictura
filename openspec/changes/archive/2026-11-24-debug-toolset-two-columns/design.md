# Design

The panel's column layout holds the header, then a horizontal layout of two vertical
layouts (each ending in a stretch so tables keep their natural height), then the size
grip row. `addGroup` already takes the layout to append to, so groups only change
which column they are given. The right column carries the three smaller tables so the
two columns end at similar heights. The minimum width is dropped in favour of the
content's own.
