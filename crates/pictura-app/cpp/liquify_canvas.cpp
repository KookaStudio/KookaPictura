#include "liquify_canvas.h"

#include <QtGui/QAction>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QTabletEvent>
#include <QtGui/QTransform>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QMenu>

#include <algorithm>
#include <cmath>

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

// The Zoom tool menu's levels in CS6's groups; 0 separates.
constexpr double kZoomLevels[] = {0.0625, 0.125, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0,
                                  1.0,    0,     2.0,  3.0,       4.0, 6.0,       8.0,
                                  10.0,   12.0,  16.0};

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
    if (dims.size() == 2) {
        layerWidth_ = dims[0];
        layerHeight_ = dims[1];
    }
    // CS6 opens Liquify at 100%, centred on the layer.
    zoom_ = 1.0;
    offset_ = QPointF((width() - layerWidth_) / 2.0, (height() - layerHeight_) / 2.0);
    placed_ = width() > 0 && height() > 0;
    viewChanged();
}

void LiquifyCanvas::refreshAll()
{
    meshDirty_ = true;
    const double dpr = devicePixelRatioF();
    const int w = static_cast<int>(std::ceil(width() * dpr));
    const int h = static_cast<int>(std::ceil(height() * dpr));
    const QPointF origin = toLayer(QPointF(0, 0));
    view_ = liquify_render(origin.x(), origin.y(), zoom_ * dpr, w, h);
    view_.setDevicePixelRatio(dpr);
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

void LiquifyCanvas::setShowImage(bool show)
{
    showImage_ = show;
    update();
}

void LiquifyCanvas::setMesh(bool show, double spacing, const QColor& color)
{
    showMesh_ = show;
    meshSpacing_ = spacing;
    meshColor_ = color;
    meshDirty_ = true;
    update();
}

void LiquifyCanvas::setBackdrop(const QImage& image, BackdropMode mode, double opacity)
{
    backdrop_ = image;
    backdropMode_ = mode;
    backdropOpacity_ = std::clamp(opacity, 0.0, 1.0);
    update();
}

QPainterPath LiquifyCanvas::meshPathForTest()
{
    rebuildMesh();
    return QTransform::fromTranslate(offset_.x(), offset_.y()).scale(zoom_, zoom_).map(meshPath_);
}

void LiquifyCanvas::rebuildMesh()
{
    if (!meshDirty_) {
        return;
    }
    meshDirty_ = false;
    meshPath_ = QPainterPath();
    const QPointF a = toLayer(QPointF(0, 0));
    const QPointF b = toLayer(QPointF(width(), height()));
    // Trace on a lattice about three screen pixels apart, never coarser than
    // half a cell.
    const double step = std::clamp(3.0 / zoom_, 0.5, meshSpacing_ / 2.0);
    const ::rust::Vec<double> points =
        liquify_grid_lines(a.x(), a.y(), b.x(), b.y(), meshSpacing_, step);
    bool start = true;
    for (size_t i = 0; i + 1 < points.size(); i += 2) {
        if (std::isnan(points[i])) {
            start = true;
            continue;
        }
        const QPointF p(points[i], points[i + 1]);
        if (start) {
            meshPath_.moveTo(p);
            start = false;
        } else {
            meshPath_.lineTo(p);
        }
    }
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
    fit(true);
}

void LiquifyCanvas::fitOnScreen()
{
    fit(false);
}

void LiquifyCanvas::fit(bool capAtActualPixels)
{
    if (layerWidth_ <= 0 || layerHeight_ <= 0) {
        return;
    }
    double fit = std::min((width() - 2 * kMargin) / layerWidth_,
                          (height() - 2 * kMargin) / layerHeight_);
    if (capAtActualPixels) {
        fit = std::min(fit, 1.0);
    }
    zoom_ = std::clamp(fit, kMinZoom, kMaxZoom);
    offset_ = QPointF((width() - layerWidth_ * zoom_) / 2.0, (height() - layerHeight_ * zoom_) / 2.0);
    viewChanged();
}

QList<QAction*> LiquifyCanvas::zoomMenuActions(QObject* owner)
{
    QList<QAction*> actions;
    auto add = [&](const QString& text, std::function<void()> run) {
        auto* action = new QAction(text, owner);
        connect(action, &QAction::triggered, this, std::move(run));
        actions.append(action);
    };
    for (double level : kZoomLevels) {
        if (level == 0) {
            auto* separator = new QAction(owner);
            separator->setSeparator(true);
            actions.append(separator);
            continue;
        }
        add(QStringLiteral("%1%").arg(static_cast<int>(level * 100.0)),
            [this, level] { setZoom(level); });
    }
    auto* separator = new QAction(owner);
    separator->setSeparator(true);
    actions.append(separator);
    add(QStringLiteral("Actual Pixels"), [this] { setZoom(1.0); });
    add(QStringLiteral("Fit in View"), [this] { fitInView(); });
    add(QStringLiteral("Fit on Screen"), [this] { fitOnScreen(); });
    return actions;
}

void LiquifyCanvas::showZoomMenu(const QPoint& globalPos)
{
    QMenu menu(this);
    menu.setObjectName(QStringLiteral("liquifyZoomMenu"));
    menu.addActions(zoomMenuActions(&menu));
    menu.exec(globalPos);
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
    if (layerWidth_ <= 0) {
        return;
    }
    const QRectF target(toWidget(QPointF(0, 0)), QSizeF(layerWidth_ * zoom_, layerHeight_ * zoom_));
    painter.save();
    painter.setBrushOrigin(target.topLeft());
    painter.fillRect(target, checkerBrush());
    painter.restore();
    // Only the active layer is distorted; the backdrop is drawn as it is.
    const bool backdrop = !backdrop_.isNull() && backdropOpacity_ > 0.0;
    auto drawBackdrop = [&](QPainter::CompositionMode mode) {
        painter.save();
        painter.setOpacity(backdropOpacity_);
        painter.setCompositionMode(mode);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawImage(target, backdrop_);
        painter.restore();
    };
    if (backdrop && backdropMode_ == BackdropMode::Behind) {
        drawBackdrop(QPainter::CompositionMode_SourceOver);
    }
    if (showImage_ && !view_.isNull()) {
        painter.drawImage(QPointF(0, 0), view_);
    }
    if (backdrop && backdropMode_ == BackdropMode::InFront) {
        drawBackdrop(QPainter::CompositionMode_SourceOver);
    } else if (backdrop && backdropMode_ == BackdropMode::Blend) {
        // ponytail: CS6's Blend formula is unpublished; Difference shows
        // where the two pictures disagree, which is what aligning needs.
        drawBackdrop(QPainter::CompositionMode_Difference);
    }
    if (showMesh_) {
        rebuildMesh();
        painter.save();
        painter.translate(offset_);
        painter.scale(zoom_, zoom_);
        QPen pen(meshColor_, 1.0);
        pen.setCosmetic(true);
        painter.setPen(pen);
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(meshPath_);
        painter.restore();
    }
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
    if (!placed_ && layerWidth_ > 0) {
        reload();
        return;
    }
    viewChanged();
}

void LiquifyCanvas::mousePressEvent(QMouseEvent* event)
{
    setFocus(Qt::MouseFocusReason);
    if (event->button() == Qt::RightButton && effectiveMode() == Mode::Zoom) {
        showZoomMenu(event->globalPosition().toPoint());
        return;
    }
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
    const QPointF previous = hover_;
    hover_ = event->position();
    hovering_ = true;
    if (panning_) {
        offset_ = panOffset_ + (event->position() - panAnchor_);
        viewChanged();
        return;
    }
    if (stroking_) {
        strokeTo(toLayer(event->position()));
    }
    // Repaint where the brush ring was and where it is now.
    const double r = brushSize_ / 2.0 * zoom_ + 3.0;
    update(QRectF(previous - QPointF(r, r), QSizeF(2 * r, 2 * r)).toAlignedRect());
    update(QRectF(hover_ - QPointF(r, r), QSizeF(2 * r, 2 * r)).toAlignedRect());
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
        viewChanged();
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
    viewChanged();
}

void LiquifyCanvas::viewChanged()
{
    clampOffset();
    meshDirty_ = true;
    refreshAll();
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
    // Whole device pixels keep a patch aligned with the view it lands in.
    const double dpr = devicePixelRatioF();
    offset_ = QPointF(std::round(offset_.x() * dpr) / dpr, std::round(offset_.y() * dpr) / dpr);
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
    if (rect.size() != 4 || rect.at(2) <= 0 || rect.at(3) <= 0 || view_.isNull()) {
        return;
    }
    // The changed layer rectangle in device pixels, clipped to the view.
    const double dpr = view_.devicePixelRatio();
    const QRectF widgetRect(toWidget(QPointF(rect.at(0), rect.at(1))),
                            QSizeF(rect.at(2) * zoom_, rect.at(3) * zoom_));
    const QRect device = QRectF(widgetRect.topLeft() * dpr, widgetRect.size() * dpr)
                             .toAlignedRect()
                             .adjusted(-1, -1, 1, 1)
                             .intersected(view_.rect());
    if (device.isEmpty()) {
        return;
    }
    const QPointF origin = toLayer(QPointF(device.topLeft()) / dpr);
    const QImage patch =
        liquify_render(origin.x(), origin.y(), zoom_ * dpr, device.width(), device.height());
    if (patch.isNull()) {
        return;
    }
    {
        QImage& target = view_;
        target.setDevicePixelRatio(1.0);
        QPainter painter(&target);
        painter.setCompositionMode(QPainter::CompositionMode_Source);
        painter.drawImage(device.topLeft(), patch);
        painter.end();
        target.setDevicePixelRatio(dpr);
    }
    if (showMesh_) {
        // The mesh is traced across the frame, so it repaints whole.
        meshDirty_ = true;
        update();
        return;
    }
    update(QRectF(QPointF(device.topLeft()) / dpr, QSizeF(device.size()) / dpr)
               .toAlignedRect()
               .adjusted(-1, -1, 1, 1));
}

} // namespace pictura
