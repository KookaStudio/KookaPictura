#pragma once

#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtGui/QCursor>
#include <QtGui/QIcon>

namespace pictura {

// Asset id -> resource lookup. Ids are file names without the extension:
// command ids verbatim ("file.saveAs") and tools/cursors as "tool.<tool>".
// SVG icons are tinted at render time from the active palette; `color`
// overrides the tint (an invalid color follows the palette).
QIcon icon(const QString& id);
QIcon icon(const QString& id, const QColor& color);
// Renders the cursor SVG at the given hotspot in the 24x24 cursor space.
QCursor cursor(const QString& id, int hotX, int hotY);
// As above, turned `degrees` clockwise about the hotspot.
QCursor cursor(const QString& id, int hotX, int hotY, double degrees);
// Compatibility overload: centres the hotspot at (12, 12).
QCursor cursor(const QString& id);

// The curved double-arrow for a rotate gesture: `cursor.rotate` turned so its
// arc bulges from `centre` toward `pointer` (both points in the same space),
// cached per 5 degrees so a drag does not re-render the SVG on every move.
QCursor rotateCursor(const QPointF& centre, const QPointF& pointer);

} // namespace pictura
