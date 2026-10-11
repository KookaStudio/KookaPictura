// ToolController guide drags: with the Move tool, or any tool while Ctrl is
// held, a press on a shown, unlocked guide drags it instead of reaching the
// tool. The release commits "Move Guide", or "Delete Guide" off the canvas.

#include "tools.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/guides.cxxqt.h"

#include <QtGui/QGuiApplication>

#include <cmath>
#include <utility>

namespace pictura {

namespace {

// How close, in screen pixels, a press must be to pick up a guide.
constexpr double kGuidePickPx = 4.0;

} // namespace

void ToolController::refreshGuides()
{
    if (!canvas_) {
        return;
    }
    QList<ImageView::GuideLine> lines;
    PictureView* v = view();
    const int count = v && v->has_document() ? guide_count(*v) : 0;
    for (int i = 0; i < count; ++i) {
        const ::rust::Vec<double> g = guide_at(*v, i);
        if (g.size() == 2) {
            lines.append({g[0] != 0.0, g[1]});
        }
    }
    canvas_->setGuides(lines);
}

int ToolController::guideAt(const QPointF& imagePos, Qt::KeyboardModifiers mods) const
{
    PictureView* v = view();
    if (!canvas_ || !v || !v->has_document() || guidesLocked_ || !canvas_->guidesVisible()
        || (active_ != ToolId::Move && !mods.testFlag(Qt::ControlModifier))) {
        return -1;
    }
    return guide_near(*v, imagePos.x(), imagePos.y(), kGuidePickPx / canvas_->zoom());
}

bool ToolController::beginGuideDrag(const QPointF& imagePos, Qt::KeyboardModifiers mods)
{
    guideDrag_ = guideAt(imagePos, mods);
    const ::rust::Vec<double> g =
        guideDrag_ >= 0 ? guide_at(*view(), guideDrag_) : ::rust::Vec<double>();
    if (g.size() != 2) {
        guideDrag_ = -1;
        return false;
    }
    guideDragVertical_ = g[0] != 0.0;
    guideDragStart_ = g[1];
    return true;
}

void ToolController::dragGuide(const QPointF& imagePos)
{
    PictureView* v = view();
    const double position = std::round(guideDragVertical_ ? imagePos.x() : imagePos.y());
    if (v && move_guide(*v, guideDrag_, position, false)) {
        refreshGuides();
    }
}

void ToolController::releaseGuide(const QPointF& imagePos)
{
    PictureView* v = view();
    const int index = std::exchange(guideDrag_, -1);
    if (!v || !canvas_) {
        return;
    }
    if (!QRectF(canvas_->rect()).contains(canvas_->imageToWidget(imagePos))) {
        move_guide(*v, index, guideDragStart_, false);
        remove_guide(*v, index);
    } else {
        const double position = std::round(guideDragVertical_ ? imagePos.x() : imagePos.y());
        // A click that leaves the guide where it was records nothing.
        move_guide(*v, index, position, position != guideDragStart_);
    }
    refreshGuides();
}

void ToolController::updateGuideHover(const QPointF& imagePos)
{
    const int index = guideAt(imagePos, QGuiApplication::queryKeyboardModifiers());
    if (index < 0) {
        if (std::exchange(guideHover_, false)) {
            refreshCursor();
        }
        return;
    }
    guideHover_ = true;
    const ::rust::Vec<double> g = guide_at(*view(), index);
    const bool vertical = g.size() == 2 && g[0] != 0.0;
    canvas_->setCursor(vertical ? Qt::SplitHCursor : Qt::SplitVCursor);
}

} // namespace pictura
