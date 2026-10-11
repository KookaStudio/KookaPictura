#include "tools.h"

#include "icons.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

namespace pictura {

namespace {

// Free Transform's rotate handle (`transform_hit_test`'s ROTATE_HANDLE).
constexpr int kRotateHandle = 8;

// The centre of a `transform_quad` string ("x,y x,y x,y x,y"), or `fallback`.
QPointF quadCentre(const QString& quad, const QPointF& fallback)
{
    const QStringList corners = quad.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (corners.size() != 4) {
        return fallback;
    }
    QPointF sum;
    for (const QString& corner : corners) {
        const QStringList xy = corner.split(QLatin1Char(','));
        if (xy.size() != 2) {
            return fallback;
        }
        sum += QPointF(xy[0].toDouble(), xy[1].toDouble());
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

bool ToolController::beginTransformImpl(const QString& path, const QString& mode)
{
    PictureView* v = view();
    const bool ok = mode.isEmpty() ? (v && v->begin_free_transform(path))
                                   : (v && v->begin_transform_mode(path, mode));
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
        canvas_->setPanEnabled(false);
        canvas_->setFocus();
    }
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
    canvas_->setTransformQuad(v->transform_quad());
    const QString projective = v->transform_preview_matrix();
    if (!projective.isEmpty()) {
        canvas_->setTransformPreviewProjective(projective);
    } else {
        canvas_->setTransformPreview(v->transform_scale_x(), v->transform_scale_y(),
                                     v->transform_angle(), v->transform_dx(), v->transform_dy());
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
    case kRotateHandle:
        // In widget space, so a turned view (Rotate View) turns the arrow too.
        canvas_->setCursor(
            rotateCursor(canvas_->imageToWidget(quadCentre(v->transform_quad(), imagePos)),
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
    v->commit_transform();
    transformDragging_ = false;
    transformHandle_ = -1;
    if (canvas_) {
        canvas_->clearTransformPreview();
    }
    applyToolPolicy();
}

void ToolController::cancelFreeTransform()
{
    PictureView* v = view();
    if (!v || !v->transform_session_active()) {
        return;
    }
    v->cancel_transform();
    transformDragging_ = false;
    transformHandle_ = -1;
    if (canvas_) {
        canvas_->clearTransformPreview();
    }
    applyToolPolicy();
}

} // namespace pictura
