// Selection-move drag state machine for ToolController: pressing inside a live
// selection translates the mask instead of starting a new marquee. Split from
// tools.cpp to keep each translation unit under the size cap.

#include "tools.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

namespace pictura {

bool ToolController::isSelectionTool(ToolId id)
{
    return id == ToolId::Marquee || id == ToolId::EllipticalMarquee || id == ToolId::Lasso
        || id == ToolId::PolygonalLasso || id == ToolId::MagneticLasso;
}

bool ToolController::maybeBeginSelectionMove(PictureView* v, const QPointF& imagePos)
{
    if (!v || !v->has_selection()
        || v->selection_coverage(qRound(imagePos.x()), qRound(imagePos.y())) <= 0
        || !v->begin_selection_move()) {
        return false;
    }
    contentMove_ = false;
    movingSelection_ = true;
    dragging_ = true;
    dragCommitted_ = false;
    selectionMoveAnchor_ = imagePos;
    refreshCursor();
    return true;
}

void ToolController::beginContentMove(PictureView* v, const QPointF& imagePos, bool duplicate)
{
    if (!v || !v->begin_selection_move()) {
        return;
    }
    contentMove_ = true;
    contentDuplicate_ = duplicate;
    movingSelection_ = true;
    dragging_ = true;
    dragCommitted_ = false;
    selectionMoveAnchor_ = imagePos;
    contentPreviewActive_ = false;
    // Alt copies the selected pixels to a new layer; show that copy following
    // the pointer during the drag instead of only the selection outline. With a
    // live selection `begin_move_duplicate` prepares the masked copy as the
    // preview layer from a clone, so nothing is recorded until release.
    if (duplicate && canvas_ && v->begin_move_duplicate()) {
        const QImage previewBase = v->move_preview_base();
        const QImage previewLayer = v->move_preview_layer();
        if (!previewBase.isNull() && !previewLayer.isNull()) {
            canvas_->beginMovePreview(previewBase, previewLayer,
                                      QPointF(v->move_preview_x(), v->move_preview_y()),
                                      v->move_preview_opacity() / 255.0);
            contentPreviewActive_ = true;
        }
    }
    refreshCursor();
}

void ToolController::cancelSelectionMove()
{
    contentMove_ = false;
    contentDuplicate_ = false;
    if (contentPreviewActive_ && canvas_) {
        canvas_->endMovePreview();
        contentPreviewActive_ = false;
    }
    if (!movingSelection_) {
        return;
    }
    movingSelection_ = false;
    if (canvas_) {
        canvas_->setSelectionContourOffset(QPointF());
    }
    PictureView* v = view();
    if (v) {
        v->cancel_selection_move();
    }
}

void ToolController::updateSelectionHover(const QPointF& imagePos)
{
    PictureView* v = view();
    cursorOverSelection_ = isSelectionTool(active_) && v && v->has_selection()
        && v->selection_coverage(qRound(imagePos.x()), qRound(imagePos.y())) > 0;
    refreshCursor();
}

void ToolController::dragSelectionMove(const QPointF& imagePos)
{
    PictureView* v = view();
    if (!v) {
        return;
    }
    const int dx = qRound(imagePos.x() - selectionMoveAnchor_.x());
    const int dy = qRound(imagePos.y() - selectionMoveAnchor_.y());
    if (v->preview_selection_move(dx, dy) && canvas_) {
        canvas_->setSelectionContourOffset(QPointF(dx, dy));
    }
    if (contentPreviewActive_ && canvas_) {
        canvas_->setMovePreviewDelta(QPointF(dx, dy));
    }
}

void ToolController::releaseSelectionMove(const QPointF& imagePos)
{
    if (!movingSelection_) {
        return;
    }
    movingSelection_ = false;
    // The commit retraces the moved mask; a refused or zero move shows the
    // origin contour again.
    if (canvas_) {
        canvas_->setSelectionContourOffset(QPointF());
    }
    PictureView* v = view();
    const int dx = qRound(imagePos.x() - selectionMoveAnchor_.x());
    const int dy = qRound(imagePos.y() - selectionMoveAnchor_.y());
    if (contentMove_) {
        contentMove_ = false;
        if (v && (dx != 0 || dy != 0)) {
            if (v->move_selection_content(dx, dy, contentDuplicate_)) {
                emit selectionCommitted();
            } else {
                v->cancel_selection_move();
            }
        } else if (v) {
            v->cancel_selection_move();
        }
        if (contentPreviewActive_ && canvas_) {
            canvas_->endMovePreview();
            contentPreviewActive_ = false;
        }
        contentDuplicate_ = false;
        refreshCursor();
        return;
    }
    if (v && (dx != 0 || dy != 0)) {
        if (v->preview_selection_move(dx, dy) && v->commit_selection_move()) {
            emit selectionCommitted();
        }
    } else if (v) {
        v->cancel_selection_move();
    }
    refreshCursor();
}

} // namespace pictura
