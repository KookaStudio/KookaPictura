#include "tool_region_drag.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QObject>
#include <QtGui/QGuiApplication>

namespace pictura {

bool RegionDragToolHandler::onPress(ToolServices& ctx, const QPointF& imagePos,
                                    Qt::KeyboardModifiers mods)
{
    PictureView* v = ctx.view();
    if (!v || !v->has_document()) {
        return true;
    }
    if (v->has_selection()
        && v->selection_coverage(qRound(imagePos.x()), qRound(imagePos.y())) > 0) {
        regionDrag_ = true;
        anchor_ = imagePos;
        ImageView* canvas = ctx.canvas();
        outline_ = canvas ? canvas->selectionContours() : QList<QPolygonF>();
    } else {
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        if (!v->begin_lasso(selectionModeString(ctx.dragMode()))) {
            return true;
        }
        regionDrag_ = false;
        trace_.clear();
        trace_ << imagePos;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setSelectionShapePreview({trace_});
        }
    }
    ctx.setDragging(true);
    ctx.setDragCommitted(false);
    return true;
}

void RegionDragToolHandler::onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers)
{
    if (!ctx.dragging()) {
        return;
    }
    ImageView* canvas = ctx.canvas();
    if (regionDrag_) {
        const QPointF delta(qRound(imagePos.x() - anchor_.x()),
                            qRound(imagePos.y() - anchor_.y()));
        QList<QPolygonF> moved;
        for (const QPolygonF& loop : outline_) {
            moved << loop.translated(delta);
        }
        if (canvas) {
            canvas->setSelectionPreview(moved);
        }
        return;
    }
    if (PictureView* v = ctx.view()) {
        v->lasso_add_point(qRound(imagePos.x()), qRound(imagePos.y()));
    }
    trace_ << imagePos;
    if (canvas) {
        canvas->setSelectionShapePreview({trace_});
    }
}

void RegionDragToolHandler::onRelease(ToolServices& ctx, const QPointF& imagePos,
                                  Qt::KeyboardModifiers)
{
    if (!ctx.dragging()) {
        return;
    }
    ctx.setDragging(false);
    if (ImageView* canvas = ctx.canvas()) {
        canvas->clearSelectionPreview();
    }
    PictureView* v = ctx.view();
    if (!v) {
        return;
    }
    if (!regionDrag_) {
        // CS6's Patch bar has no Feather: the outline is always hard-edged.
        const bool committed = trace_.size() >= 3 && v->end_lasso(0.0);
        if (!committed) {
            v->cancel_lasso();
        }
        trace_.clear();
        if (committed) {
            ctx.emitSelectionCommitted();
        }
        return;
    }
    regionDrag_ = false;
    outline_.clear();
    const int dx = qRound(imagePos.x() - anchor_.x());
    const int dy = qRound(imagePos.y() - anchor_.y());
    if (dx == 0 && dy == 0) {
        return;
    }
    if (activePixelLocked(v)) {
        ctx.refused(QObject::tr("Could not %1: the layer's pixels are locked.").arg(verb()));
        return;
    }
    // The solve runs on the GUI thread; a large selection takes long enough
    // to need a wait cursor.
    QGuiApplication::setOverrideCursor(Qt::WaitCursor);
    const bool applied = apply(ctx, *v, dx, dy);
    QGuiApplication::restoreOverrideCursor();
    if (!applied) {
        ctx.refused(
            QObject::tr("Could not %1: select a single pixel layer first.").arg(verb()));
    }
}

void RegionDragToolHandler::onDeactivate(ToolServices& ctx)
{
    if (ctx.dragging() && !regionDrag_) {
        if (PictureView* v = ctx.view()) {
            v->cancel_lasso();
        }
    }
    if (ImageView* canvas = ctx.canvas()) {
        canvas->clearSelectionPreview();
    }
    regionDrag_ = false;
    outline_.clear();
    trace_.clear();
}

} // namespace pictura
