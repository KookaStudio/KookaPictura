#include "tools.h"

#include "api/transform_api.h"
#include "icons.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/warp.cxxqt.h"

#include <QtCore/QStringList>

#include <cmath>

namespace pictura {

namespace {

// Free Transform's rotate handle (`transform_hit_test`'s ROTATE_HANDLE).
constexpr int kRotateHandle = 8;
// The reference point's drag handle (`PIVOT_HANDLE`).
constexpr int kPivotHandle = 10;

// The centre of a transform quad's four document-space corners, or `fallback`.
QPointF quadCentre(const transform_api::Quad& quad, const QPointF& fallback)
{
    if (quad.corners.size() != 4) {
        return fallback;
    }
    QPointF sum;
    for (const QPointF& corner : quad.corners) {
        sum += corner;
    }
    return sum / 4.0;
}

} // namespace

bool ToolController::beginFreeTransform(const QString& path)
{
    return beginTransformImpl(path, QString());
}

bool ToolController::beginTransformMode(const QString& path, const QString& mode)
{
    return beginTransformImpl(path, mode);
}

bool ToolController::setTransformMode(const QString& mode)
{
    PictureView* v = view();
    if (!v || !transform_set_mode(*v, mode)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::applyTransformOp(const QString& op, bool commit, const QString& label)
{
    PictureView* v = view();
    if (!v || !transform_apply(*v, op)) {
        return false;
    }
    if (commit) {
        transform_set_history_label(*v, label);
        commitFreeTransform();
    } else {
        updateTransformOverlay(v);
        emit transformChanged();
    }
    return true;
}

bool ToolController::repeatTransform()
{
    PictureView* v = view();
    if (!v || !transform_repeat(*v)) {
        return false;
    }
    transform_set_history_label(*v, QStringLiteral("Transform Again"));
    commitFreeTransform();
    return true;
}

bool ToolController::beginTransformImpl(const QString& path, const QString& mode, bool controls)
{
    PictureView* v = view();
    if (!v) {
        return false;
    }
    if (transformControls_ && !controls) {
        // A user command takes the Move tool's controls box over as a modal
        // session (the same path would otherwise be a begin no-op).
        transformControls_ = false;
        clearControlsBox();
        v->cancel_transform();
    }
    const bool ok = mode.isEmpty() ? v->begin_free_transform(path)
                                   : v->begin_transform_mode(path, mode);
    if (!ok) {
        return false;
    }
    transform_set_controls(*v, controls);
    transformControls_ = controls;
    transformGuideSnap_ = false;
    if (canvas_) {
        canvas_->beginTransformPreview(v->move_preview_base(), v->move_preview_layer(),
                                      QPointF(v->move_preview_x(), v->move_preview_y()),
                                      v->move_preview_opacity() / 255.0);
        updateTransformOverlay(v);
        // A transform session is modal: drop any transient Space pan so its
        // pan-disable is not swallowed by the override's pan guard.
        canvas_->setSpacePan(false);
        canvas_->setPanEnabled(false);
        // Only the modal command takes the keyboard. The Move tool's controls
        // box is rebuilt on every refresh and must not pull focus from a text
        // field.
        if (!controls) {
            canvas_->setFocus();
        }
    }
    emit transformChanged();
    return true;
}

bool ToolController::transformSessionActive() const
{
    PictureView* v = view();
    return v && v->transform_session_active();
}

void ToolController::updateTransformOverlay(PictureView* v)
{
    if (!canvas_ || !v) {
        return;
    }
    canvas_->setTransformQuad(transform_api::quad(*v).corners);
    const bool warping = transform_warp_active(*v);
    const bool warpStored = transform_warp_stored(*v);
    // Warp mode has no reference point: the grid replaces the box.
    canvas_->setTransformPivot(QPointF(transform_pivot_x(*v), transform_pivot_y(*v)),
                               transform_show_pivot(*v) && !warping);
    if (warpStored) {
        // The moving layer already carries the warp and the frame; it is a new
        // image after every re-warp, so the canvas must be handed the latest.
        canvas_->updateTransformPreviewLayer(v->move_preview_layer(),
                                             QPointF(v->move_preview_x(), v->move_preview_y()));
    }
    if (warping) {
        canvas_->setTransformWarp(transform_api::warpOverlay(*v));
    } else {
        canvas_->setTransformWarp(transform_api::WarpOverlay());
    }
    const transform_api::Matrix matrix = transform_api::previewLocalMatrix(*v);
    if (matrix.valid) {
        // A preview layer with a stored warp already carries the warp and frame,
        // so it is drawn whole (an empty source); otherwise sample the box.
        QRectF source;
        const QStringList parts = warpStored
            ? QStringList()
            : transform_session_rect(*v).split(QLatin1Char(' '));
        if (parts.size() == 4) {
            const double x = v->move_preview_x();
            const double y = v->move_preview_y();
            source = QRectF(parts[0].toDouble() - x, parts[1].toDouble() - y,
                            parts[2].toDouble() - parts[0].toDouble(),
                            parts[3].toDouble() - parts[1].toDouble());
        }
        canvas_->setTransformPreviewProjective(matrix.coefficients, source);
    } else {
        canvas_->setTransformPreview(1.0, 1.0, 0.0, 0.0, 0.0);
    }
}

bool ToolController::resolvePendingTransform()
{
    if (!transformPending()) {
        return true;
    }
    switch (transformPrompt_ ? transformPrompt_() : TransformDecision::DontApply) {
    case TransformDecision::Apply:
        commitFreeTransform();
        return true;
    case TransformDecision::DontApply:
        cancelFreeTransform();
        return true;
    case TransformDecision::Cancel:
        return false;
    }
    return false;
}

void ToolController::updateTransformReadout(PictureView* v, const QPointF& imagePos)
{
    if (!canvas_) {
        return;
    }
    const double angle = v ? transform_drag_angle_degrees(*v) : std::nan("");
    if (std::isfinite(angle)) {
        canvas_->setDragSizeHint(QStringLiteral("∠: %1°").arg(angle, 0, 'f', 1),
                                 imagePos);
    } else {
        canvas_->clearDragSizeHint();
    }
}

void ToolController::setTransformCursor(const QPointF& imagePos)
{
    PictureView* v = view();
    if (!canvas_ || !v) {
        return;
    }
    // A drag keeps the cursor of the handle it grabbed, wherever the pointer goes.
    const int dragged = v->transform_drag_handle();
    const int hit = dragged >= 0
        ? dragged
        : v->transform_hit_test(imagePos.x(), imagePos.y(), canvas_->zoom());
    setHandleCursor(hit, imagePos, quadCentre(transform_api::quad(*v), imagePos));
}

void ToolController::setHandleCursor(int hit, const QPointF& imagePos, const QPointF& centre)
{
    switch (hit) {
    case 0:
    case 2:
        canvas_->setCursor(Qt::SizeFDiagCursor);
        break;
    case 1:
    case 3:
        canvas_->setCursor(Qt::SizeBDiagCursor);
        break;
    case 4:
    case 6:
        canvas_->setCursor(Qt::SizeVerCursor);
        break;
    case 5:
    case 7:
        canvas_->setCursor(Qt::SizeHorCursor);
        break;
    case 9: {
        PictureView* v = view();
        canvas_->setCursor(v && v->transform_drag_handle() >= 0 ? Qt::ClosedHandCursor
                                                                : Qt::OpenHandCursor);
        break;
    }
    case kPivotHandle:
        canvas_->setCursor(Qt::SizeAllCursor);
        break;
    case kRotateHandle:
        // In widget space, so a turned view (Rotate View) turns the arrow too.
        canvas_->setCursor(
            rotateCursor(canvas_->imageToWidget(centre), canvas_->imageToWidget(imagePos)));
        break;
    default:
        canvas_->setCursor(Qt::ArrowCursor);
        break;
    }
}

void ToolController::commitFreeTransform()
{
    PictureView* v = view();
    if (!v || !v->transform_session_active()) {
        return;
    }
    transformControls_ = false;
    v->commit_transform();
    transformGuideSnap_ = false;
    resetTransformUi();
}

void ToolController::cancelFreeTransform()
{
    PictureView* v = view();
    if (!v || !v->transform_session_active()) {
        return;
    }
    transformControls_ = false;
    v->cancel_transform();
    transformGuideSnap_ = false;
    resetTransformUi();
}

void ToolController::resetTransformUi()
{
    transformControls_ = false;
    transformGuideSnap_ = false;
    clearControlsBox();
    if (canvas_) {
        canvas_->clearTransformPreview();
        canvas_->clearSmartGuides();
    }
    applyToolPolicy();
    emit transformChanged();
}

// The Move tool's `Show Transform Controls` box. Idle, it is only drawn: the
// box round the active layer's content (`controls_box_extents`), the layer
// untouched, so filters and other edits see its full extent. A press on one of
// its handles starts a Free Transform session for that one gesture, applied on
// release (`handleReleased`), after which the box is idle again. Hidden for any
// other tool, an unchecked box, an active selection, or an untransformable
// layer; rebuilt when the target layer, its rect, or the history step changed.
void ToolController::syncTransformControls()
{
    PictureView* v = view();
    if (!v) {
        transformControls_ = false;
        clearControlsBox();
        return;
    }
    // A modal (Ctrl+T) session owns the overlay; never touch it here.
    if (v->transform_session_active() && !transformControls_) {
        return;
    }
    // A gesture in progress keeps its session until the release applies it.
    if (transformControls_ && v->transform_session_active()) {
        return;
    }
    const bool want = active_ == ToolId::Move && move_->get().showTransformControls
        && !v->has_selection();
    const QString active = layer_selection_anchor(*v);
    // The content scan runs only when the layer, its rect, or the history step
    // changed; a refresh or a press on an unchanged box reuses the last one.
    const QString key = want && !active.isEmpty()
        ? QStringLiteral("%1 %2 %3").arg(active, transform_api::layerRect(*v, active).toString())
              .arg(v->history_index())
        : QString();
    if (controlsIdle_ && !key.isEmpty() && key == controlsKey_) {
        return;
    }
    const ::rust::Vec<int32_t> box = !key.isEmpty() && v->layer_can_free_transform(active)
        ? controls_box_extents(*v, active)
        : ::rust::Vec<int32_t>();
    if (box.size() != 4) {
        if (transformControls_) {
            transformControls_ = false;
            clearControlsBox();
            emit transformChanged();
        }
        return;
    }
    const bool was = transformControls_;
    transformControls_ = true;
    controlsIdle_ = true;
    controlsPath_ = active;
    controlsKey_ = key;
    controlsBox_ = QRect(QPoint(box[0], box[1]), QPoint(box[2] - 1, box[3] - 1));
    if (canvas_) {
        canvas_->setControlsBox(QPolygonF(QRectF(QPointF(box[0], box[1]), QPointF(box[2], box[3]))));
    }
    if (!was) {
        emit transformChanged();
    }
}

int ToolController::controlsHit(const QPointF& imagePos) const
{
    return controls_box_hit_test(controlsBox_.left(), controlsBox_.top(), controlsBox_.right() + 1,
                                 controlsBox_.bottom() + 1, imagePos.x(), imagePos.y(),
                                 canvas_ ? canvas_->zoom() : 1.0);
}

void ToolController::clearControlsBox()
{
    controlsIdle_ = false;
    controlsKey_.clear();
    if (canvas_) {
        canvas_->setControlsBox(QPolygonF());
    }
}

bool ToolController::beginControlsGesture(PictureView* v)
{
    clearControlsBox();
    if (!beginTransformImpl(controlsPath_, QString(), true)) {
        transformControls_ = false;
        return false;
    }
    // CS6's controls box has no reference point (that is the modal command's).
    transform_set_show_pivot(*v, false);
    updateTransformOverlay(v);
    return true;
}

bool ToolController::setTransformPivot(double x, double y)
{
    PictureView* v = view();
    if (!v || !transform_set_pivot(*v, x, y)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformPivotPreset(int anchor)
{
    PictureView* v = view();
    if (!v || !transform_set_pivot_preset(*v, anchor)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

void ToolController::setTransformShowPivot(bool on)
{
    PictureView* v = view();
    if (!v) {
        return;
    }
    transform_set_show_pivot(*v, on);
    updateTransformOverlay(v);
    emit transformChanged();
}

bool ToolController::setTransformWidth(double pct, bool link)
{
    PictureView* v = view();
    if (!v || !transform_set_width(*v, pct, link)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformHeight(double pct, bool link)
{
    PictureView* v = view();
    if (!v || !transform_set_height(*v, pct, link)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformAngle(double degrees)
{
    PictureView* v = view();
    if (!v || !transform_set_angle(*v, degrees)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformSkew(double hDegrees, double vDegrees)
{
    PictureView* v = view();
    if (!v || !transform_set_skew(*v, hDegrees, vDegrees)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformInterpolation(const QString& id)
{
    PictureView* v = view();
    if (!v || !transform_set_interpolation(*v, id)) {
        return false;
    }
    emit transformChanged();
    return true;
}

bool ToolController::beginWarp(const QString& path)
{
    if (!transformSessionActive() && !beginFreeTransform(path)) {
        return false;
    }
    return enterTransformWarp();
}

bool ToolController::enterTransformWarp()
{
    PictureView* v = view();
    if (!v || !transform_enter_warp(*v)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::leaveTransformWarp()
{
    PictureView* v = view();
    if (!v || !transform_leave_warp(*v)) {
        return false;
    }
    if (canvas_) {
        canvas_->updateTransformPreviewLayer(v->move_preview_layer(),
                                             QPointF(v->move_preview_x(), v->move_preview_y()));
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformWarpStyle(const QString& id)
{
    PictureView* v = view();
    if (!v || !transform_set_warp_style(*v, id)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformWarpValues(double bend, double hDistort, double vDistort)
{
    PictureView* v = view();
    if (!v || !transform_set_warp_values(*v, bend, hDistort, vDistort)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

bool ToolController::setTransformWarpVertical(bool vertical)
{
    PictureView* v = view();
    if (!v || !transform_set_warp_vertical(*v, vertical)) {
        return false;
    }
    updateTransformOverlay(v);
    emit transformChanged();
    return true;
}

} // namespace pictura
