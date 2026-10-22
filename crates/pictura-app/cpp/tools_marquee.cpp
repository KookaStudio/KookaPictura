// Marquee/lasso geometry and overlay helpers for ToolController. Split from
// tools.cpp to keep each translation unit under the size cap.

#include "tools.h"

#include "icons.h"
#include "image_view.h"
#include "selection_geometry.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_path.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtGui/QCursor>
#include <QtGui/QGuiApplication>
#include <QtGui/QPainterPath>

#include <algorithm>
#include <cmath>

namespace pictura {

bool activePixelLocked(PictureView* view)
{
    if (!view) {
        return false;
    }
    return (layer_lock_path(*view, view->active_layer_path()) & 0x02) != 0;
}

void ToolController::refreshCursor()
{
    refreshCursor(QGuiApplication::queryKeyboardModifiers());
}

void ToolController::refreshCursor(Qt::KeyboardModifiers mods)
{
    if (!canvas_) {
        return;
    }
    // A held Space owns the cursor: the transient hand override must win over
    // every tool, hover, and sampler update until it is released.
    if (canvas_->spacePanForTest()) {
        return canvas_->setCursor(canvas_->isPanning() ? Qt::ClosedHandCursor
                                                       : Qt::OpenHandCursor);
    }
    if (canvasSampler_) {
        const ToolInfo& info = toolInfo(ToolId::Eyedropper);
        const QCursor c = cursor(toolCursorId(ToolId::Eyedropper, Qt::NoModifier), info.hotspotX,
                                 info.hotspotY);
        return canvas_->setCursor(c.pixmap().isNull() ? QCursor(info.cursor) : c);
    }
    PictureView* hoverView = view();
    const bool ctrlPreview = isSelectionTool(active_) && hoverView
        && mods.testFlag(Qt::ControlModifier) && hoverView->has_selection();
    // An in-progress combine drag keeps the cursor captured at press for the
    // whole gesture, even after the live modifier is released.
    if (isSelectionTool(active_) && dragging_ && !movingSelection_) {
        const ToolInfo& info = toolInfo(active_);
        const QCursor c = cursor(dragCursorId(), info.hotspotX, info.hotspotY);
        if (!c.pixmap().isNull()) {
            return canvas_->setCursor(c);
        }
    }
    // A combine modifier (Shift/Alt) hands the cursor back to the tool's
    // add/remove assets; hovering without a drag still offers move-selection.
    if (hoverCursorId(active_, mods, movingSelection_ || cursorOverSelection_, ctrlPreview)
        == QStringLiteral("cursor.moveSelection")) {
        const QCursor c = cursor(QStringLiteral("cursor.moveSelection"), 2, 2);
        if (!c.pixmap().isNull()) {
            return canvas_->setCursor(c);
        }
    }
    if (isBrushTool(active_)) {
        // The transient Alt eyedropper wins over the blank paint cursor and the
        // invisible/locked refusal (tool-framework cursor precedence).
        // On the Clone Stamp, Alt sets the source point instead; on the
        // Eraser it erases to history, so the paint cursor stays.
        if (mods.testFlag(Qt::AltModifier) && active_ == ToolId::CloneStamp) {
            return canvas_->setCursor(Qt::CrossCursor);
        }
        if (mods.testFlag(Qt::AltModifier) && active_ != ToolId::Eraser) {
            const ToolInfo& info = toolInfo(ToolId::Eyedropper);
            const QCursor c =
                cursor(toolCursorId(ToolId::Eyedropper, mods), info.hotspotX, info.hotspotY);
            return canvas_->setCursor(c.pixmap().isNull() ? QCursor(info.cursor) : c);
        }
        if (hoverView
            && (activePixelLocked(hoverView) || !hoverView->active_layer_visible())) {
            return canvas_->setCursor(Qt::ForbiddenCursor);
        }
        // The drawn brush-size ring is the pointer affordance; hide the OS
        // cursor rather than scaling a pixmap with the brush.
        return canvas_->setCursor(Qt::BlankCursor);
    }
    // A handler that owns a pointer-driven cursor (Crop) reports the state
    // cursor for the live pointer; otherwise the tool's static cursor is used.
    if (ToolHandler* h = registry_.forTool(active_)) {
        const QPoint local = canvas_->mapFromGlobal(QCursor::pos());
        QCursor hover;
        if (h->hoverCursor(*this, canvas_->widgetToImage(QPointF(local)), mods, hover)) {
            return canvas_->setCursor(hover);
        }
    }
    const ToolInfo& info = toolInfo(active_);
    ToolHandler* h = registry_.forTool(active_);
    cursorVariant_ = h ? h->cursorVariant() : QString();
    const QCursor toolCursor =
        cursor(toolCursorId(active_, mods) + cursorVariant_, info.hotspotX, info.hotspotY);
    canvas_->setCursor(toolCursor.pixmap().isNull() ? QCursor(info.cursor) : toolCursor);
}

namespace {

// The synthetic modifier set that selects a combine mode's cursor asset, so a
// locked drag mode can reuse `toolCursorId` while live keys are ignored.
Qt::KeyboardModifiers modsForSelectionMode(SelectionMode mode)
{
    switch (mode) {
    case SelectionMode::Add:
        return Qt::ShiftModifier;
    case SelectionMode::Subtract:
        return Qt::AltModifier;
    case SelectionMode::Intersect:
        return Qt::ShiftModifier | Qt::AltModifier;
    case SelectionMode::New:
        break;
    }
    return Qt::NoModifier;
}

} // namespace

QString ToolController::hoverCursorId(ToolId id, Qt::KeyboardModifiers mods, bool overSelection,
                                      bool ctrlPreview)
{
    if (isSelectionTool(id) && !mods.testFlag(Qt::ShiftModifier)
        && !mods.testFlag(Qt::AltModifier) && (overSelection || ctrlPreview)) {
        return QStringLiteral("cursor.moveSelection");
    }
    return toolCursorId(id, mods);
}

QString ToolController::dragCursorId() const
{
    return toolCursorId(active_, modsForSelectionMode(dragMode_));
}

QString ToolController::cursorIdForModifiersForTest(ToolId id, int mods) const
{
    return toolCursorId(id, Qt::KeyboardModifiers(mods));
}

SelectionMode ToolController::selectionModeForModifiers(SelectionMode base,
                                                        Qt::KeyboardModifiers mods,
                                                        bool hasExistingSelection)
{
    const bool shift = mods.testFlag(Qt::ShiftModifier);
    const bool alt = mods.testFlag(Qt::AltModifier);
    if (!shift && !alt) {
        return base;
    }
    if (!hasExistingSelection) {
        return SelectionMode::New;
    }
    if (shift && alt) {
        return SelectionMode::Intersect;
    }
    if (shift) {
        return SelectionMode::Add;
    }
    return SelectionMode::Subtract;
}

QRect ToolController::marqueeRectForTest(const QPointF& a, const QPointF& b, int mods,
                                         bool mirror) const
{
    const SelectionOptions* selection = optionsFor<SelectionOptions>(ToolId::Marquee);
    return marqueeDragRect(a, b, Qt::KeyboardModifiers(mods), mirror, selection->marqueeStyle(),
                           selection->fixedRatioWidth(), selection->fixedRatioHeight(),
                           selection->fixedSizeWidth(), selection->fixedSizeHeight());
}

void ToolController::updateBrushOutline(const QPointF& imagePos)
{
    if (!canvas_) {
        return;
    }
    if (!isBrushTool(active_)) {
        canvas_->clearBrushOutline();
        return;
    }
    if (active_ == ToolId::Eraser && eraserBlockMode()) {
        // The Block's cursor is a square of the size the block erases.
        canvas_->setBlockOutline(eraser_block_side_doc_px(canvas_->zoom()), imagePos);
        return;
    }
    canvas_->setBrushOutline(paint_->size(), imagePos);
}

void ToolController::refreshBrushOutline()
{
    if (!canvas_ || !isBrushTool(active_)) {
        return;
    }
    const QPoint local = canvas_->mapFromGlobal(QCursor::pos());
    if (canvas_->rect().contains(local)) {
        updateBrushOutline(canvas_->widgetToImage(QPointF(local)));
        return;
    }
    // The pointer is not over the canvas, so the outline is either gone or
    // left where it was: re-apply it there, or leave the canvas alone.
    if (canvas_->hasBrushOutline()) {
        updateBrushOutline(canvas_->brushOutlineImagePos());
    }
}

bool ToolController::eraserBlockMode() const
{
    const EraserGroup* group = optionsFor<EraserGroup>(ToolId::Eraser);
    if (!group || group->get().mode != 2) {
        return false;
    }
    // A mask erase paints the mask's coverage with the brush tip, so the
    // Block's square is not in effect there and the ring stays a circle.
    PictureView* v = view();
    return !v || mask_edit_target_get(*v).isEmpty();
}

} // namespace pictura
