#include "tools.h"

#include "api/transform_api.h"
#include "icons.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"

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
        v->cancel_transform();
    }
    const bool ok = mode.isEmpty() ? v->begin_free_transform(path)
                                   : v->begin_transform_mode(path, mode);
    if (!ok) {
        return false;
    }
    transformDragging_ = false;
    transformHandle_ = -1;
    if (canvas_) {
        canvas_->beginTransformPreview(v->move_preview_base(), v->move_preview_layer(),
                                      QPointF(v->move_preview_x(), v->move_preview_y()),
                                      v->move_preview_opacity() / 255.0);
        updateTransformOverlay(v);
        // A transform session is modal: drop any transient Space pan so its
        // pan-disable is not swallowed by the override's pan guard.
        canvas_->setSpacePan(false);
        canvas_->setPanEnabled(false);
        canvas_->setFocus();
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
    canvas_->setTransformPivot(QPointF(transform_pivot_x(*v), transform_pivot_y(*v)),
                               transform_show_pivot(*v));
    const transform_api::Matrix matrix = transform_api::previewMatrix(*v);
    if (matrix.valid) {
        canvas_->setTransformPreviewProjective(matrix.coefficients);
    } else {
        canvas_->setTransformPreview(1.0, 1.0, 0.0, 0.0, 0.0);
    }
}

void ToolController::setTransformCursor(const QPointF& imagePos)
{
    PictureView* v = view();
    if (!canvas_ || !v) {
        return;
    }
    // A drag keeps the cursor of the handle it grabbed, wherever the pointer goes.
    const int hit = transformDragging_
        ? transformHandle_
        : v->transform_hit_test(imagePos.x(), imagePos.y(), canvas_->zoom());
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
    case 9:
        canvas_->setCursor(transformDragging_ ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        break;
    case kPivotHandle:
        canvas_->setCursor(Qt::SizeAllCursor);
        break;
    case kRotateHandle:
        // In widget space, so a turned view (Rotate View) turns the arrow too.
        canvas_->setCursor(
            rotateCursor(canvas_->imageToWidget(quadCentre(transform_api::quad(*v), imagePos)),
                         canvas_->imageToWidget(imagePos)));
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
    transformDragging_ = false;
    transformHandle_ = -1;
    if (canvas_) {
        canvas_->clearTransformPreview();
    }
    applyToolPolicy();
    emit transformChanged();
}

void ToolController::cancelFreeTransform()
{
    PictureView* v = view();
    if (!v || !v->transform_session_active()) {
        return;
    }
    transformControls_ = false;
    v->cancel_transform();
    transformDragging_ = false;
    transformHandle_ = -1;
    if (canvas_) {
        canvas_->clearTransformPreview();
    }
    applyToolPolicy();
    emit transformChanged();
}

// The Move tool's `Show Transform Controls` box: an auto-managed Free
// Transform session on the active layer, applied per gesture. Hidden for any
// other tool, an unchecked box, an active selection, or an untransformable
// layer; rebuilt when the target layer or its content rect changed.
void ToolController::syncTransformControls()
{
    PictureView* v = view();
    if (!v) {
        transformControls_ = false;
        return;
    }
    // A modal (Ctrl+T) session owns the overlay; never touch it here.
    if (v->transform_session_active() && !transformControls_) {
        return;
    }
    const bool want = active_ == ToolId::Move && move_->get().showTransformControls
        && !v->has_selection();
    if (!want) {
        if (transformControls_) {
            transformControls_ = false;
            v->cancel_transform();
            if (canvas_) {
                canvas_->clearTransformPreview();
            }
            emit transformChanged();
        }
        return;
    }
    const QString active = v->active_layer_path();
    if (transformControls_ && v->transform_session_active()
        && v->transform_session_path() == active
        && transform_session_rect(*v) == transform_api::layerRect(*v, active).toString()) {
        return;
    }
    if (transformControls_ && v->transform_session_active()) {
        // A stale box (a content edit, an undo, another layer): rebuild it.
        v->cancel_transform();
        if (canvas_) {
            canvas_->clearTransformPreview();
        }
    }
    transformControls_ = false;
    if (active.isEmpty() || !v->layer_can_free_transform(active)) {
        return;
    }
    transformControls_ = true;
    if (!beginTransformImpl(active, QString(), true)) {
        transformControls_ = false;
        return;
    }
    // CS6's controls box has no reference point (that is the modal command's).
    transform_set_show_pivot(*v, false);
    updateTransformOverlay(v);
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

bool ToolController::setTransformWarp(const QString& styleId, double bend, double dx, double dy,
                                      bool vertical)
{
    PictureView* v = view();
    if (!v || !transform_set_warp(*v, styleId, bend, dx, dy, vertical)) {
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

void ToolController::clearTransformWarp()
{
    PictureView* v = view();
    if (!v || !transform_clear_warp(*v)) {
        return;
    }
    if (canvas_) {
        canvas_->updateTransformPreviewLayer(v->move_preview_layer(),
                                             QPointF(v->move_preview_x(), v->move_preview_y()));
    }
    updateTransformOverlay(v);
    emit transformChanged();
}

} // namespace pictura
