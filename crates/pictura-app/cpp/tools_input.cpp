#include "tools.h"

#include "api/transform_api.h"
#include "api/transform_bridge.h"
#include "image_view.h"

#include <QtGui/QGuiApplication>

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

void ToolController::handleModifiers(int modifiers)
{
    emit modifiersChanged(modifiers);
    if (canvasBrush_ || canvasSampler_ || transformSessionActive() || movingSelection_) {
        return;
    }
    if (ToolHandler* h = registry_.forTool(active_)) {
        h->onModifiers(*this, Qt::KeyboardModifiers(modifiers));
    }
}

void ToolController::handleReleased(const QPointF& imagePos)
{
    if (ignoreRelease_ || ignoreSingleLineRelease()) {
        return;
    }
    if (temporarySampling_) {
        temporarySampling_ = false;
        registry_.forTool(ToolId::Eyedropper)->onRelease(*this, imagePos, Qt::NoModifier);
        refreshCursor();
        return;
    }
    if (canvasBrush_) {
        canvasBrushDragging_ = false;
        canvasBrush_(CanvasBrushPhase::Release, imagePos,
                     QGuiApplication::queryKeyboardModifiers());
        return;
    }
    if (canvasSampler_) {
        canvasSampling_ = false;
        return;
    }
    if (transformSessionActive()) {
        PictureView* v = view();
        if (v && v->transform_drag_handle() >= 0) {
            transformGuideSnap_ = false;
            v->transform_release();
            if (canvas_) {
                canvas_->clearSmartGuides();
                canvas_->clearDragSizeHint();
                // The released anchor's handles go back to near-pointer.
                if (transform_warp_active(*v)) {
                    canvas_->setTransformWarp(transform_api::warpOverlay(*v));
                }
            }
            if (transformControlsSession()) {
                if (transform_is_identity(*v)) {
                    // A press/release (or a drag that returned to the start)
                    // changed nothing: drop the session and restore the box.
                    cancelFreeTransform();
                } else {
                    promoteControlsTransform(*v);
                }
            }
        }
        return;
    }
    if (guideDrag_ >= 0) {
        releaseGuide(imagePos);
        return;
    }
    if (movingSelection_) {
        dragging_ = false;
        releaseSelectionMove(imagePos);
        return;
    }
    if (ToolHandler* h = registry_.forTool(active_); h && !sliceEditingBlocked()) {
        h->onRelease(*this, imagePos, QGuiApplication::queryKeyboardModifiers());
        if (h->pointerCursor() || h->cursorVariant() != cursorVariant_) {
            refreshCursor();
        }
    }
    // A suspended controls box (an inside/outside Move drag) returns here, even
    // when the drag changed nothing.
    syncTransformControls();
}

} // namespace pictura
