#include "api/paint_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QTimer>

#include <utility>

namespace pictura::paint_api {

Stroke::Stroke(PictureView& view, std::function<bool()> begin)
    : view_(&view)
{
    if (begin) {
        open_ = begin();
    }
}

Stroke::~Stroke()
{
    finish();
}

Stroke::Stroke(Stroke&& other) noexcept
    : view_(other.view_)
    , open_(other.open_)
{
    other.view_ = nullptr;
    other.open_ = false;
}

Stroke& Stroke::operator=(Stroke&& other) noexcept
{
    if (this != &other) {
        finish();
        view_ = other.view_;
        open_ = other.open_;
        other.view_ = nullptr;
        other.open_ = false;
    }
    return *this;
}

bool Stroke::dab(double x, double y, double pressure)
{
    if (!open_ || !view_) {
        return false;
    }
    return view_->paint_dab(x, y, pressure);
}

bool Stroke::hold()
{
    if (!open_ || !view_) {
        return false;
    }
    return paint_hold(*view_);
}

bool Stroke::present()
{
    if (!open_ || !view_) {
        return false;
    }
    return view_->flush_present();
}

bool Stroke::finish()
{
    if (!open_ || !view_) {
        return false;
    }
    open_ = false;
    return view_->end_paint();
}

void Stroke::cancel()
{
    if (!open_ || !view_) {
        return;
    }
    open_ = false;
    view_->cancel_paint();
}

void Stroke::abandon()
{
    open_ = false;
}

std::optional<Stroke> beginBrush(PictureView& view, std::uint32_t foreground,
                                 std::uint32_t background, const PaintTip& tip, int opacity,
                                 int flow, const QString& mode, bool aliased, bool autoErase)
{
    Stroke stroke(view, [&] {
        return begin_brush(view, foreground, background, tip, opacity, flow, mode, aliased,
                           autoErase);
    });
    if (!stroke.open()) {
        return std::nullopt;
    }
    return std::optional<Stroke>(std::move(stroke));
}

void presentDeferred(PictureView& view)
{
    PictureView* target = &view;
    QTimer::singleShot(0, target, [target] { target->flush_present(); });
}

} // namespace pictura::paint_api
