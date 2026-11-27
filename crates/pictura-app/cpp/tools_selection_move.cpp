// Selection-move drag state machine for ToolController: pressing inside a live
// selection translates the mask instead of starting a new marquee. Split from
// tools.cpp to keep each translation unit under the size cap.

#include "tools.h"

#include "file_drop_router.h"
#include "image_view.h"
#include "selection_drag.h"
#include "tool_handler.h"

#include "api/selection_api.h"
#include "api/selection_bridge.h"

#include <QtGui/QDrag>
#include <QtWidgets/QApplication>

namespace pictura {

namespace {

bool isSingleLineMarquee(ToolId id)
{
    return id == ToolId::SingleRowMarquee || id == ToolId::SingleColumnMarquee;
}

} // namespace

bool ToolController::ignoreSingleLineRelease() const
{
    return canvas_ && canvas_->nonLeftReleaseWhileLeftHeld() && isSingleLineMarquee(active_);
}

bool ToolController::cancelSingleLineMarquee()
{
    if (!isSingleLineMarquee(active_)) {
        return false;
    }
    if (movingSelection_ && !contentMove_) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onEscape();
        }
        dragging_ = false;
        ignoreReleaseUntilPress();
        cancelSelectionMove(boundView_);
        return true;
    }
    if (ToolHandler* handler = registry_.forTool(active_)) {
        return handler->onEscape();
    }
    return false;
}

bool ToolController::handleEscape()
{
    if (temporarySampling_) {
        temporarySampling_ = false;
        registry_.forTool(ToolId::Eyedropper)->onSessionCancelled(*this);
        refreshCursor();
        return true;
    }
    if (isSingleLineMarquee(active_)) {
        return cancelSingleLineMarquee();
    }
    if (ToolHandler* handler = registry_.forTool(active_)) {
        return handler->onEscape();
    }
    return false;
}

bool ToolController::isSelectionTool(ToolId id)
{
    return id == ToolId::Marquee || id == ToolId::EllipticalMarquee
        || id == ToolId::SingleRowMarquee || id == ToolId::SingleColumnMarquee || id == ToolId::Lasso
        || id == ToolId::PolygonalLasso || id == ToolId::MagneticLasso;
}

bool ToolController::isSelectionMenuTool(ToolId id)
{
    return isSelectionTool(id) || id == ToolId::MagicWand || id == ToolId::QuickSelection;
}

bool ToolController::maybeBeginSelectionMove(PictureView* v, const QPointF& imagePos)
{
    if (!v || !v->has_selection()
        || v->selection_coverage(qRound(imagePos.x()), qRound(imagePos.y())) <= 0
        || !pictura::begin_selection_move(*v)) {
        return false;
    }
    contentMove_ = false;
    movingSelection_ = true;
    selectionMoveDragged_ = false;
    dragging_ = true;
    dragCommitted_ = false;
    selectionMoveAnchor_ = imagePos;
    if (isSingleLineMarquee(active_)) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onPress(*this, imagePos, Qt::NoModifier);
        }
    }
    refreshCursor();
    return true;
}

void ToolController::beginContentMove(PictureView* v, const QPointF& imagePos, bool duplicate)
{
    if (!v || !pictura::begin_selection_move(*v)) {
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
                                      v->move_preview_opacity() / 255.0,
                                      transform_api::previewBaseOrigin(*v));
            contentPreviewActive_ = true;
        }
    }
    refreshCursor();
}

void ToolController::cancelSelectionMove(PictureView* target)
{
    if (movingSelection_ && !contentMove_ && isSingleLineMarquee(active_)) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onEscape();
        }
        if (canvas_) {
            canvas_->clearSelectionPreview();
        }
    }
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
    PictureView* v = target ? target : view();
    if (v) {
        pictura::cancel_selection_move(*v);
        emit selectionMovePreviewed();
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
    if (!contentMove_ && canvas_
        && FileDropRouter::otherDocumentAt(
               canvas_->mapToGlobal(canvas_->imageToWidget(imagePos)).toPoint(), v, canvas_)) {
        dragSelectionToDocument();
        return;
    }
    if (canvas_ && (canvas_->imageToWidget(imagePos)
                    - canvas_->imageToWidget(selectionMoveAnchor_)).manhattanLength()
            >= QApplication::startDragDistance()) {
        selectionMoveDragged_ = true;
    }
    const int dx = qRound(imagePos.x() - selectionMoveAnchor_.x());
    const int dy = qRound(imagePos.y() - selectionMoveAnchor_.y());
    if (!contentMove_ && canvas_ && isSingleLineMarquee(active_)
        && selectionMoveDragged_) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            if (handler->onEscape()) {
                ignoreRelease_ = false;
                dragging_ = true;
            }
        }
    }
    if (pictura::preview_selection_move(*v, dx, dy) && canvas_) {
        canvas_->setSelectionContourOffset(QPointF(dx, dy));
        emit selectionMovePreviewed();
    }
    if (contentPreviewActive_ && canvas_) {
        canvas_->setMovePreviewDelta(QPointF(dx, dy));
    }
}

void ToolController::dragSelectionToDocument()
{
    PictureView* v = view();
    ImageView* canvas = canvas_;
    if (!v || !canvas) {
        return;
    }
    if (!cancelSingleLineMarquee()) {
        cancelSelectionMove();
    }
    dragging_ = false;
    auto* drag = new QDrag(canvas);
    drag->setMimeData(makeSelectionDragMime(v));
    // Blocks until the drop; the router may switch the active document, so
    // neither the controller's view nor the canvas is touched afterwards.
    drag->exec(Qt::CopyAction);
}

bool ToolController::prepareSelectionDocumentDragForTest()
{
    if (!movingSelection_ || contentMove_) {
        return false;
    }
    if (!cancelSingleLineMarquee()) {
        cancelSelectionMove();
    }
    dragging_ = false;
    return true;
}

void ToolController::releaseSelectionMove(const QPointF& imagePos)
{
    if (!movingSelection_) {
        return;
    }
    movingSelection_ = false;
    const int dx = qRound(imagePos.x() - selectionMoveAnchor_.x());
    const int dy = qRound(imagePos.y() - selectionMoveAnchor_.y());
    const bool lineClick = !contentMove_ && isSingleLineMarquee(active_)
        && dx == 0 && dy == 0 && !selectionMoveDragged_;
    if (!contentMove_ && isSingleLineMarquee(active_) && !lineClick) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onEscape();
            ignoreRelease_ = false;
        }
    }
    // The commit retraces the moved mask; a refused or zero move shows the
    // origin contour again.
    if (canvas_) {
        canvas_->setSelectionContourOffset(QPointF());
    }
    PictureView* v = view();
    if (contentMove_) {
        contentMove_ = false;
        if (v && (dx != 0 || dy != 0)) {
            // Apply the release offset first: a release can arrive without a
            // final pointer-move event, and the selection must follow the move.
            pictura::preview_selection_move(*v, dx, dy);
            if (pictura::move_selection_content(*v, dx, dy, contentDuplicate_,
                                                quint32(background_.rgba()))) {
                emit selectionCommitted();
            } else {
                pictura::cancel_selection_move(*v);
            }
        } else if (v) {
            pictura::cancel_selection_move(*v);
        }
        if (contentPreviewActive_ && canvas_) {
            canvas_->endMovePreview();
            contentPreviewActive_ = false;
        }
        contentDuplicate_ = false;
        emit selectionMovePreviewed();
        refreshCursor();
        return;
    }
    if (v && (dx != 0 || dy != 0)) {
        if (pictura::preview_selection_move(*v, dx, dy) && pictura::commit_selection_move(*v)) {
            emit selectionCommitted();
        } else {
            pictura::cancel_selection_move(*v);
            emit selectionMovePreviewed();
        }
    } else if (v) {
        pictura::cancel_selection_move(*v);
        emit selectionMovePreviewed();
    }
    if (lineClick && v) {
        if (ToolHandler* handler = registry_.forTool(active_)) {
            handler->onRelease(*this, selectionMoveAnchor_, Qt::NoModifier);
        }
    }
    refreshCursor();
}

} // namespace pictura
