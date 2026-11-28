#include "tools.h"

#include "image_view.h"
#include "tool_handler.h"

#include <algorithm>
#include <cmath>

namespace pictura {

void ToolController::setPressure(double pressure)
{
    pressure_ = std::clamp(pressure, 0.0, 1.0);
}

void ToolController::setPointerPose(double tiltX, double tiltY, double rotation, double wheel)
{
    tiltX_ = std::clamp(tiltX, -90.0, 90.0);
    tiltY_ = std::clamp(tiltY, -90.0, 90.0);
    rotation_ = std::fmod(rotation, 360.0);
    if (rotation_ < 0.0) {
        rotation_ += 360.0;
    }
    wheel_ = std::clamp(wheel, 0.0, 1.0);
}

void ToolController::bindCanvas(ImageView* canvas)
{
    // A document switch or mode conversion can make the active tool
    // unavailable; Move is always allowed.
    if (!toolAllowedForDocument(active_, view())) {
        setActiveTool(ToolId::Move);
    }
    if (canvas_ == canvas) {
        boundView_ = view();
        // The canvas pointer is unchanged, but the active layer's visibility or
        // identity may have changed under it; re-assert the cursor without
        // waiting for the next mouse move.
        if (!transformSessionActive()) {
            refreshCursor();
        }
        if (ToolHandler* h = registry_.forTool(active_)) {
            h->onDocumentRefreshed(*this);
        }
        refreshAnnotations();
        // Every refresh reaches here: keep the Move tool's controls box on the
        // active layer (or rebuild it after a content change).
        syncTransformControls();
        return;
    }
    unbindCanvas();
    canvas_ = canvas;
    boundView_ = view();
    // The current note belongs to the previous document.
    setCurrentNote(-1);
    if (!canvas_) {
        return;
    }
    connect(canvas_, &ImageView::mousePressed, this, &ToolController::handlePressed);
    connect(canvas_, &ImageView::mouseDoubleClicked, this, [this](const QPointF& pos, int mods, bool* handled) {
        if (ToolHandler* h = registry_.forTool(active_); h && !sliceEditingBlocked()) {
            *handled = h->onDoubleClick(*this, pos, Qt::KeyboardModifiers(mods));
        }
    });
    connect(canvas_, &ImageView::pointerPressure, this, &ToolController::setPressure);
    connect(canvas_, &ImageView::pointerPose, this, &ToolController::setPointerPose);
    connect(canvas_, &ImageView::mouseMoved, this, &ToolController::handleMoved);
    connect(canvas_, &ImageView::modifierKeyChanged, this, &ToolController::handleModifiers);
    connect(canvas_, &ImageView::mouseReleased, this, &ToolController::handleReleased);
    connect(canvas_, &ImageView::pointerGestureFocusLost, this, &ToolController::cancelSingleLineMarquee);
    connect(canvas_, &ImageView::transformCommitRequested, this,
            &ToolController::commitFreeTransform);
    connect(canvas_, &ImageView::transformCancelRequested, this,
            &ToolController::cancelFreeTransform);
    // The Eraser's Block cursor is sized in document pixels, so a zoom change
    // resizes it.
    connect(canvas_, &ImageView::zoomChanged, this, [this](double) { refreshBrushOutline(); });
    applyToolPolicy();
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onDocumentRefreshed(*this);
    }
    refreshAnnotations();
}

} // namespace pictura
