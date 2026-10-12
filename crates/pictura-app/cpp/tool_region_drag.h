#pragma once

#include "tool_handler.h"

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtGui/QPolygonF>

namespace pictura {

class PictureView;

// CS6's two-step region gesture shared by Patch and Content-Aware Move. A drag
// outside the selection traces a freehand outline (combined with the selection
// like the Lasso, never feathered); a drag that starts inside it shows the
// outline following the pointer and, on a release that moved, hands the drag
// to `apply` under a wait cursor. Ported from photorust's CanvasView healing
// press/drag/release.
class RegionDragToolHandler : public ToolHandler {
public:
    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override;
    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override;
    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override;
    void onDeactivate(ToolServices& ctx) override;

protected:
    // Run the tool on the selection dragged by `(dx, dy)`; false refuses.
    virtual bool apply(ToolServices& ctx, PictureView& view, int dx, int dy) = 0;
    // The refusal verb, e.g. "patch" in "Could not patch: ...".
    virtual QString verb() const = 0;

private:
    bool regionDrag_ = false;
    QPointF anchor_;
    QList<QPolygonF> outline_;
    QPolygonF trace_;
};

} // namespace pictura
