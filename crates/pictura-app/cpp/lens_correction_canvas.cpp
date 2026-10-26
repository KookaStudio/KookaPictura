#include "lens_correction_canvas.h"

#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QWheelEvent>

#include <algorithm>
#include <cmath>

#include "pictura_app/src/cxxqt_object/filter_tools/lens_correction.cxxqt.h"

namespace pictura {

namespace {

constexpr double kMinZoom = 0.0625;
constexpr double kMaxZoom = 16.0;
// How much ground stays between a panned picture and the frame edge.
constexpr double kMargin = 24.0;

const QBrush& checkerBrush()
{
    static const QBrush brush = [] {
        QPixmap tile(16, 16);
        tile.fill(Qt::white);
        QPainter p(&tile);
        p.fillRect(0, 0, 8, 8, QColor(0xcc, 0xcc, 0xcc));
        p.fillRect(8, 8, 8, 8, QColor(0xcc, 0xcc, 0xcc));
        return QBrush(tile);
    }();
    return brush;
}

} // namespace

LensCorrectionCanvas::LensCorrectionCanvas(QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("lcCanvas"));
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_OpaquePaintEvent);
    setMinimumSize(320, 240);
    updateCursor();
}

void LensCorrectionCanvas::reload()
{
    const ::rust::Vec<int32_t> dims = lc_geometry();
    if (dims.size() == 2) {
        layerWidth_ = dims[0];
        layerHeight_ = dims[1];
    }
    // CS6 opens the dialog at 100%, centred.
    zoom_ = 1.0;
    offset_ = QPointF((width() - layerWidth_) / 2.0, (height() - layerHeight_) / 2.0);
    placed_ = width() > 0 && height() > 0;
    viewChanged();
}

void LensCorrectionCanvas::refresh()
{
    const double dpr = devicePixelRatioF();
    const int w = static_cast<int>(std::ceil(width() * dpr));
    const int h = static_cast<int>(std::ceil(height() * dpr));
    const QPointF origin = toDisplay(QPointF(0, 0));
    view_ = lc_render(origin.x(), origin.y(), zoom_ * dpr, w, h, preview_);
    view_.setDevicePixelRatio(dpr);
    update();
}

void LensCorrectionCanvas::setMode(Mode mode)
{
    mode_ = mode;
    updateCursor();
}

void LensCorrectionCanvas::setPreview(bool on)
{
    preview_ = on;
    refresh();
}

void LensCorrectionCanvas::setShowGrid(bool on)
{
    showGrid_ = on;
    update();
}

void LensCorrectionCanvas::setGridSize(int size)
{
    gridSize_ = std::max(size, 1);
    gridOffset_ = QPointF(std::fmod(gridOffset_.x(), gridSize_), std::fmod(gridOffset_.y(), gridSize_));
    update();
}

void LensCorrectionCanvas::setGridColor(const QColor& color)
{
    gridColor_ = color;
    update();
}

void LensCorrectionCanvas::setZoom(double zoom)
{
    zoomAbout(zoom, QPointF(width() / 2.0, height() / 2.0));
}

void LensCorrectionCanvas::zoomBy(double factor)
{
    setZoom(zoom_ * factor);
}

QPointF LensCorrectionCanvas::toDisplay(const QPointF& widget) const
{
    return (widget - offset_) / zoom_;
}

QPointF LensCorrectionCanvas::toWidget(const QPointF& display) const
{
    return offset_ + display * zoom_;
}

QList<double> LensCorrectionCanvas::gridLinesForTest() const
{
    QList<double> lines;
    for (double x = gridOffset_.x(); x <= layerWidth_; x += gridSize_) {
        lines.append(x);
    }
    for (double y = gridOffset_.y(); y <= layerHeight_; y += gridSize_) {
        lines.append(y);
    }
    return lines;
}

double LensCorrectionCanvas::radius(const QPointF& display) const
{
    return std::hypot(display.x() - layerWidth_ / 2.0, display.y() - layerHeight_ / 2.0);
}

void LensCorrectionCanvas::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(0xa8, 0xa8, 0xa8));
    if (layerWidth_ <= 0) {
        return;
    }
    const QRectF target(toWidget(QPointF(0, 0)), QSizeF(layerWidth_ * zoom_, layerHeight_ * zoom_));
    painter.save();
    painter.setBrushOrigin(target.topLeft());
    painter.fillRect(target, checkerBrush());
    painter.restore();
    if (!view_.isNull()) {
        painter.drawImage(QPointF(0, 0), view_);
    }
    if (showGrid_) {
        painter.save();
        painter.setClipRect(target);
        QPen pen(gridColor_, 1.0);
        pen.setCosmetic(true);
        painter.setPen(pen);
        for (double x = gridOffset_.x(); x <= layerWidth_; x += gridSize_) {
            const double at = toWidget(QPointF(x, 0)).x();
            painter.drawLine(QPointF(at, target.top()), QPointF(at, target.bottom()));
        }
        for (double y = gridOffset_.y(); y <= layerHeight_; y += gridSize_) {
            const double at = toWidget(QPointF(0, y)).y();
            painter.drawLine(QPointF(target.left(), at), QPointF(target.right(), at));
        }
        painter.restore();
    }
    if (drag_ == Drag::Line) {
        QPen pen(Qt::black, 1.0, Qt::DashLine);
        pen.setCosmetic(true);
        painter.setPen(pen);
        painter.drawLine(toWidget(lineStart_), toWidget(lineEnd_));
    }
}

void LensCorrectionCanvas::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    if (!placed_ && layerWidth_ > 0) {
        reload();
        return;
    }
    viewChanged();
}

void LensCorrectionCanvas::mousePressEvent(QMouseEvent* event)
{
    setFocus(Qt::MouseFocusReason);
    const QPointF pos = event->position();
    pressWidget_ = pos;
    const Mode mode = event->button() == Qt::MiddleButton ? Mode::Hand : effectiveMode();
    if (event->button() == Qt::RightButton) {
        return;
    }
    switch (mode) {
    case Mode::Hand:
        drag_ = Drag::Pan;
        panOffset_ = offset_;
        setCursor(Qt::ClosedHandCursor);
        break;
    case Mode::Zoom:
        zoomAbout(zoom_ * ((event->modifiers() & Qt::AltModifier) ? 0.5 : 2.0), pos);
        break;
    case Mode::RemoveDistortion:
        drag_ = Drag::Distort;
        distortStart_ = distortionValue ? distortionValue() : 0.0;
        break;
    case Mode::Straighten:
        drag_ = Drag::Line;
        lineStart_ = lineEnd_ = toDisplay(pos);
        break;
    case Mode::MoveGrid:
        drag_ = Drag::Grid;
        gridStart_ = gridOffset_;
        break;
    }
}

void LensCorrectionCanvas::mouseMoveEvent(QMouseEvent* event)
{
    const QPointF pos = event->position();
    switch (drag_) {
    case Drag::Pan:
        offset_ = panOffset_ + (pos - pressWidget_);
        viewChanged();
        break;
    case Drag::Distort: {
        // A drag across the half-diagonal toward the centre is +100.
        const double half = std::hypot(layerWidth_, layerHeight_) / 2.0;
        if (half > 0.0 && distortionDragged) {
            const double delta =
                (radius(toDisplay(pressWidget_)) - radius(toDisplay(pos))) / half * 100.0;
            distortionDragged(std::clamp(distortStart_ + delta, -100.0, 100.0));
        }
        break;
    }
    case Drag::Line:
        lineEnd_ = toDisplay(pos);
        update();
        break;
    case Drag::Grid: {
        const QPointF moved = gridStart_ + (pos - pressWidget_) / zoom_;
        const double s = gridSize_;
        gridOffset_ = QPointF(moved.x() - std::floor(moved.x() / s) * s,
                              moved.y() - std::floor(moved.y() / s) * s);
        update();
        break;
    }
    case Drag::None:
        break;
    }
}

void LensCorrectionCanvas::mouseReleaseEvent(QMouseEvent* event)
{
    const Drag drag = drag_;
    drag_ = Drag::None;
    if (drag == Drag::Line) {
        lineEnd_ = toDisplay(event->position());
        const QPointF d = event->position() - pressWidget_;
        if (std::hypot(d.x(), d.y()) > 3.0 && straightened) {
            straightened(lineStart_, lineEnd_);
        }
        update();
    }
    updateCursor();
}

void LensCorrectionCanvas::wheelEvent(QWheelEvent* event)
{
    const QPoint delta = event->angleDelta();
    if (event->modifiers() & Qt::ControlModifier) {
        zoomAbout(zoom_ * (delta.y() > 0 ? 1.25 : 0.8), event->position());
    } else {
        const bool sideways = event->modifiers() & Qt::ShiftModifier;
        offset_ += sideways ? QPointF(delta.y() / 2.0, 0) : QPointF(delta.x() / 2.0, delta.y() / 2.0);
        viewChanged();
    }
    event->accept();
}

void LensCorrectionCanvas::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space) {
        if (!event->isAutoRepeat()) {
            spaceHeld_ = true;
            updateCursor();
        }
        return;
    }
    QWidget::keyPressEvent(event);
}

void LensCorrectionCanvas::keyReleaseEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space && !event->isAutoRepeat()) {
        spaceHeld_ = false;
        updateCursor();
        return;
    }
    QWidget::keyReleaseEvent(event);
}

LensCorrectionCanvas::Mode LensCorrectionCanvas::effectiveMode() const
{
    return spaceHeld_ ? Mode::Hand : mode_;
}

void LensCorrectionCanvas::updateCursor()
{
    switch (effectiveMode()) {
    case Mode::Hand:
        setCursor(Qt::OpenHandCursor);
        break;
    case Mode::MoveGrid:
        setCursor(Qt::SizeAllCursor);
        break;
    case Mode::RemoveDistortion:
    case Mode::Straighten:
    case Mode::Zoom:
        setCursor(Qt::CrossCursor);
        break;
    }
}

void LensCorrectionCanvas::zoomAbout(double zoom, const QPointF& anchor)
{
    const double next = std::clamp(zoom, kMinZoom, kMaxZoom);
    const QPointF display = toDisplay(anchor);
    zoom_ = next;
    offset_ = anchor - display * zoom_;
    viewChanged();
}

void LensCorrectionCanvas::viewChanged()
{
    clampOffset();
    refresh();
    if (zoomChanged) {
        zoomChanged();
    }
}

void LensCorrectionCanvas::clampOffset()
{
    const double w = layerWidth_ * zoom_;
    const double h = layerHeight_ * zoom_;
    auto clampAxis = [](double offset, double extent, double frame) {
        if (extent <= frame - 2 * kMargin) {
            return (frame - extent) / 2.0;
        }
        return std::clamp(offset, frame - extent - kMargin, kMargin);
    };
    offset_ = QPointF(clampAxis(offset_.x(), w, width()), clampAxis(offset_.y(), h, height()));
    const double dpr = devicePixelRatioF();
    offset_ = QPointF(std::round(offset_.x() * dpr) / dpr, std::round(offset_.y() * dpr) / dpr);
}

} // namespace pictura

#include "moc_lens_correction_canvas.cpp"
