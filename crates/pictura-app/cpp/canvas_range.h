#pragma once

#include <QtCore/QSizeF>

#include <algorithm>

namespace pictura {

// ponytail: one display inch (96 logical pixels) of the canvas must stay
// reachable, so the clamp slides each axis by the minimum instead of recentring.
// If the reveal margin ever needs to change, this constant is the only knob:
// the ImageView pan clamp and the workspace scrollbars both read it here.
inline constexpr double kCanvasRevealMarginPx = 96.0;

// Allowed widget-space canvas offset (top-left of the scaled document) so that
// at least `margin` logical pixels of each axis stay inside the viewport. A
// document smaller than the margin keeps its whole extent visible instead. When
// `freePan` is false, each axis the document fits and has content on collapses
// to the single centred offset, so the canvas is pinned there (Photoshop's
// constrained pan); an overflowing or empty axis is left on the margin range.
// When `snapToWorkspace` is true and the whole scaled document fits both axes,
// both axes collapse to the centre regardless of `freePan`, so the canvas is
// locked to the workspace centre.
struct OffsetRange {
    double minX = 0.0;
    double maxX = 0.0;
    double minY = 0.0;
    double maxY = 0.0;
};

inline OffsetRange offsetRangeFor(const QSizeF& imageSize, double zoom, const QSizeF& viewport,
                                  bool freePan = true, double margin = kCanvasRevealMarginPx,
                                  bool snapToWorkspace = false)
{
    const double w = imageSize.width() * zoom;
    const double h = imageSize.height() * zoom;
    const double visibleX = std::min(margin, w);
    const double visibleY = std::min(margin, h);

    OffsetRange range;
    range.minX = visibleX - w;
    range.maxX = viewport.width() - visibleX;
    range.minY = visibleY - h;
    range.maxY = viewport.height() - visibleY;
    if (snapToWorkspace && w > 0.0 && h > 0.0 && w <= viewport.width()
        && h <= viewport.height()) {
        // ponytail: a document-level lock, not per-axis. The whole document must
        // fit before it pins, so a wide-but-short image that overflows an axis
        // keeps the free-pan policy and pans normally.
        range.minX = range.maxX = (viewport.width() - w) / 2.0;
        range.minY = range.maxY = (viewport.height() - h) / 2.0;
    } else if (!freePan) {
        // ponytail: constrain only fires on an axis the document fits and has
        // content on; an empty doc or an overflowing axis keeps today's reveal
        // margin, so zoomed-in pan is unchanged. Negate this by making the
        // collapse unconditional for a pasteboard/overscroll mode.
        if (w > 0.0 && w <= viewport.width()) {
            range.minX = range.maxX = (viewport.width() - w) / 2.0;
        }
        if (h > 0.0 && h <= viewport.height()) {
            range.minY = range.maxY = (viewport.height() - h) / 2.0;
        }
    }
    if (range.minX > range.maxX) {
        range.minX = range.maxX = (viewport.width() - w) / 2.0;
    }
    if (range.minY > range.maxY) {
        range.minY = range.maxY = (viewport.height() - h) / 2.0;
    }
    return range;
}

} // namespace pictura
