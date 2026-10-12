#include "liquify_canvas.h"

#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QTabletEvent>
#include <QtGui/QWheelEvent>

#include <algorithm>

#include "pictura_app/src/cxxqt_object/filter_tools/liquify.cxxqt.h"

namespace pictura {

namespace {

// CS6's zoom range in the Liquify dialog.
constexpr double kMinZoom = 0.0625;
constexpr double kMaxZoom = 16.0;
// Brush Rate ticks while a held tool stays still.
constexpr int kHoldIntervalMs = 40;
// How much ground stays between a panned picture and the frame edge.
constexpr double kMargin = 24.0;

QList<int> toList(const ::rust::Vec<int32_t>& v)
{
    QList<int> out;
    for (int32_t value : v) {
        out.append(value);
    }
    return out;
}

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

LiquifyCanvas::LiquifyCanvas(QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("liquifyCanvas"));
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_OpaquePaintEvent);
    setMinimumSize(320, 240);
    holdTimer_.setInterval(kHoldIntervalMs);
    connect(&holdTimer_, &QTimer::timeout, this, [this] { hold(); });
    updateCursor();
}

void LiquifyCanvas::reload()
{
    const ::rust::Vec<int32_t> dims = liquify_geometry();
    if (dims.size() == 3) {
        layerWidth_ = dims[0];
        layerHeight_ = dims[1];
        scale_ = std::max(1, static_cast<int>(dims[2]));
    }
    refreshAll();
    if (width() > 0 && height() > 0) {
        fitInView();
    }
}

void LiquifyCanvas::refreshAll()
{
    preview_ = liquify_image().convertToFormat(QImage::Format_RGBA8888_Premultiplied);
    update();
}

void LiquifyCanvas::setMode(Mode mode, bool actsWhileHeld)
{
    mode_ = mode;
    actsWhileHeld_ = actsWhileHeld;
    updateCursor();
    update();
}

void LiquifyCanvas::setBrushSize(double size)
{
    brushSize_ = size;
    update();
}

void LiquifyCanvas::setStylusPressure(bool on)
{
    stylus_ = on;
}

void LiquifyCanvas::setZoom(double zoom)
{
    zoomAbout(zoom, QPointF(width() / 2.0, height() / 2.0));
}

void LiquifyCanvas::zoomBy(double factor)
{
    setZoom(zoom_ * factor);
}

void LiquifyCanvas::fitInView()
{
    if (layerWidth_ <= 0 || layerHeight_ <= 0) {
        return;
    }
    const double fit = std::min((width() - 2 * kMargin) / layerWidth_,
                                (height() - 2 * kMargin) / layerHeight_);
    zoom_ = std::clamp(std::min(fit, 1.0), kMinZoom, kMaxZoom);
    offset_ = QPointF((width() - layerWidth_ * zoom_) / 2.0, (height() - layerHeight_ * zoom_) / 2.0);
    fitted_ = true;
    clampOffset();
    update();
    if (zoomChanged) {
        zoomChanged();
    }
}

QPointF LiquifyCanvas::toLayer(const QPointF& pos) const
{
    return (pos - offset_) / zoom_;
}

QPointF LiquifyCanvas::toWidget(const QPointF& layer) const
{
    return offset_ + layer * zoom_;
}

void LiquifyCanvas::strokeForTest(const QList<QPointF>& layerPoints, bool reverse, int ticks)
{
    if (layerPoints.isEmpty()) {
        return;
    }
    beginStroke(layerPoints.first(), reverse, false);
    for (qsizetype i = 1; i < layerPoints.size(); ++i) {
        strokeTo(layerPoints.at(i));
    }
    for (int i = 0; i < ticks; ++i) {
        hold();
    }
    stroking_ = false;
    holdTimer_.stop();
    hasLastClick_ = true;
}

void LiquifyCanvas::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(0xa8, 0xa8, 0xa8));
    if (preview_.isNull() || layerWidth_ <= 0) {
        return;
    }
    const QRectF target(toWidget(QPointF(0, 0)), QSizeF(layerWidth_ * zoom_, layerHeight_ * zoom_));
    painter.save();
    painter.setBrushOrigin(target.topLeft());
    painter.fillRect(target, checkerBrush());
    painter.restore();
    // Smooth while the preview is minified or barely magnified; at high zoom
    // the pixels show square, as on the canvas.
    painter.setRenderHint(QPainter::SmoothPixmapTransform, zoom_ * scale_ < 2.0);
    const QRectF source(0, 0, static_cast<double>(layerWidth_) / scale_,
                        static_cast<double>(layerHeight_) / scale_);
    painter.drawImage(target, preview_, source);

    if (effectiveMode() == Mode::Brush && hovering_) {
        const double r = brushSize_ / 2.0 * zoom_;
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(0, 0, 0, 160), 3.0));
        painter.drawEllipse(hover_, r, r);
        painter.setPen(QPen(QColor(255, 255, 255), 1.0));
        painter.drawEllipse(hover_, r, r);
    }
}

void LiquifyCanvas::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    if (!fitted_) {
        fitInView();
    } else {
        clampOffset();
    }
}

void LiquifyCanvas::mousePressEvent(QMouseEvent* event)
{
    setFocus(Qt::MouseFocusReason);
    const Mode mode = event->button() == Qt::MiddleButton ? Mode::Hand : effectiveMode();
    if (mode == Mode::Hand) {
        panning_ = true;
        panAnchor_ = event->position();
        panOffset_ = offset_;
        setCursor(Qt::ClosedHandCursor);
        return;
    }
    if (event->button() != Qt::LeftButton) {
        return;
    }
    if (mode == Mode::Zoom) {
        const bool out = event->modifiers() & Qt::AltModifier;
        zoomAbout(zoom_ * (out ? 0.5 : 2.0), event->position());
        return;
    }
    beginStroke(toLayer(event->position()), event->modifiers() & Qt::AltModifier,
                (event->modifiers() & Qt::ShiftModifier) && hasLastClick_);
}

void LiquifyCanvas::mouseMoveEvent(QMouseEvent* event)
{
    hover_ = event->position();
    hovering_ = true;
    if (panning_) {
        offset_ = panOffset_ + (event->position() - panAnchor_);
        clampOffset();
    } else if (stroking_) {
        strokeTo(toLayer(event->position()));
    }
    update();
}

void LiquifyCanvas::mouseReleaseEvent(QMouseEvent*)
{
    if (panning_) {
        panning_ = false;
        updateCursor();
    }
    if (stroking_) {
        stroking_ = false;
        holdTimer_.stop();
        hasLastClick_ = true;
    }
}

void LiquifyCanvas::wheelEvent(QWheelEvent* event)
{
    const QPoint delta = event->angleDelta();
    if (event->modifiers() & Qt::ControlModifier) {
        zoomAbout(zoom_ * (delta.y() > 0 ? 1.25 : 0.8), event->position());
    } else {
        const bool sideways = event->modifiers() & Qt::ShiftModifier;
        offset_ += sideways ? QPointF(delta.y() / 2.0, 0) : QPointF(delta.x() / 2.0, delta.y() / 2.0);
        clampOffset();
        update();
    }
    event->accept();
}

void LiquifyCanvas::tabletEvent(QTabletEvent* event)
{
    if (tabletSeen) {
        tabletSeen();
        tabletSeen = nullptr;
    }
    tabletPressure_ = event->pressure();
    tabletDown_ = event->type() != QEvent::TabletRelease;
    // Qt synthesizes the mouse events the strokes run on.
    event->ignore();
}

void LiquifyCanvas::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space && !event->isAutoRepeat()) {
        spaceHeld_ = true;
        updateCursor();
        update();
        return;
    }
    QWidget::keyPressEvent(event);
}

void LiquifyCanvas::keyReleaseEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space && !event->isAutoRepeat()) {
        spaceHeld_ = false;
        updateCursor();
        update();
        return;
    }
    QWidget::keyReleaseEvent(event);
}

void LiquifyCanvas::leaveEvent(QEvent* event)
{
    hovering_ = false;
    update();
    QWidget::leaveEvent(event);
}

LiquifyCanvas::Mode LiquifyCanvas::effectiveMode() const
{
    return spaceHeld_ ? Mode::Hand : mode_;
}

void LiquifyCanvas::updateCursor()
{
    switch (effectiveMode()) {
    case Mode::Hand:
        setCursor(Qt::OpenHandCursor);
        break;
    case Mode::Zoom:
    case Mode::Brush:
        setCursor(Qt::CrossCursor);
        break;
    }
}

void LiquifyCanvas::zoomAbout(double zoom, const QPointF& anchor)
{
    const double next = std::clamp(zoom, kMinZoom, kMaxZoom);
    const QPointF layer = toLayer(anchor);
    zoom_ = next;
    offset_ = anchor - layer * zoom_;
    clampOffset();
    update();
    if (zoomChanged) {
        zoomChanged();
    }
}

void LiquifyCanvas::clampOffset()
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
}

double LiquifyCanvas::pressure() const
{
    return stylus_ && tabletDown_ ? tabletPressure_ : 1.0;
}

void LiquifyCanvas::beginStroke(const QPointF& layer, bool reverse, bool straightLine)
{
    if (beforeStroke) {
        beforeStroke(reverse);
    }
    stroking_ = true;
    if (straightLine) {
        // Shift-click: a straight drag from the previous click.
        applyPatch(toList(liquify_stroke_begin(lastLayer_.x(), lastLayer_.y(), pressure())));
        strokeTo(layer);
    } else {
        applyPatch(toList(liquify_stroke_begin(layer.x(), layer.y(), pressure())));
        lastLayer_ = layer;
    }
    if (actsWhileHeld_) {
        holdTimer_.start();
    }
}

void LiquifyCanvas::strokeTo(const QPointF& layer)
{
    applyPatch(toList(liquify_stroke_to(layer.x(), layer.y(), pressure())));
    lastLayer_ = layer;
}

void LiquifyCanvas::hold()
{
    applyPatch(toList(liquify_hold()));
}

void LiquifyCanvas::applyPatch(const QList<int>& rect)
{
    if (rect.size() != 4 || rect.at(2) <= 0 || rect.at(3) <= 0 || preview_.isNull()) {
        return;
    }
    const QImage patch = liquify_patch(rect.at(0), rect.at(1), rect.at(2), rect.at(3));
    if (patch.isNull()) {
        return;
    }
    {
        QPainter painter(&preview_);
        painter.setCompositionMode(QPainter::CompositionMode_Source);
        painter.drawImage(rect.at(0), rect.at(1), patch);
    }
    const QRectF layerRect(rect.at(0) * scale_, rect.at(1) * scale_, rect.at(2) * scale_,
                           rect.at(3) * scale_);
    update(QRectF(toWidget(layerRect.topLeft()), layerRect.size() * zoom_)
               .toAlignedRect()
               .adjusted(-2, -2, 2, 2));
    if (edited) {
        edited();
    }
}

} // namespace pictura
