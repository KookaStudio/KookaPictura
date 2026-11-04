# Design

## Bounds and count caches

`PictureViewRust` keeps `((Plane::ident, width), Option<PsdRect>)` and
`(Plane::ident, i32)` in `Cell`s next to the outline cache. Two planes with one
ident hold the same bytes and every mutable borrow of a plane drops its ident,
so the key changes with every write to the mask, including one that bypasses
`set_selection` or `touch_selection`; a cloned mask (history, deselect) keeps
its ident and its cache entry. The cached bounds are the mask's own;
`selection_bounds` still offsets them by a live selection-move delta and clips,
so the preview's readout follows the drag without a scan. A compute counter,
bridged as `selection_bounds_compute_count`, lets a Qt test prove a hover does
not rescan.

`combine_selection` asked whether a New-mode commit replaces a non-empty
selection by scanning it; it asks the bounds cache instead.

## Outline window

`pictura_select::contour_within(selection, threshold, window)` traces the rows
`window.top..window.bottom` and columns `window.left..window.right`, keeping
lattice keys in document coordinates. Outside the window every pixel is below
the threshold (the bounds hold every pixel with coverage above 0), so no edge
lies outside it and the edges inside are the same. `contour` is
`contour_within` over the whole canvas. The app passes the cached bounds.

## Shape masks

`Selection::none` and `all` use `Plane::build`, which takes zeroed pages from
the allocator instead of filling a vector and copying it into the plane. `rect`
and `ellipse` start from `none` and write the unique plane in place, so a
box-sized shape touches only its rows' pages.

## Cursor cache

`pictura::cursor(id, hotX, hotY, degrees)` keys a static `QHash` by those
values plus `qApp->devicePixelRatio()`. A missing asset caches the null cursor
too. Reusing one `QCursor` keeps its pixmap's cache key, so the xcb platform
reuses the X cursor it made for it. The rotate cursor's own cache is dropped:
its 5° steps are keys of this one.

## Measurement

`marquee_selection_profile` gains the bounds read, its cached re-read, and the
windowed outline, with `PICTURA_PROFILE_DIVISOR` scaling from 16507×16196.
