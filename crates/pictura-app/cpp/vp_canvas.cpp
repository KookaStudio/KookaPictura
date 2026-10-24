#include "vp_canvas.h"

#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QPolygonF>
#include <QtGui/QWheelEvent>

#include <algorithm>
#include <cmath>
#include <vector>

#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point.cxxqt.h"

namespace pictura {

namespace {

constexpr double kMinZoom = 0.0625;
constexpr double kMaxZoom = 16.0;
// How much ground stays between a panned picture and the frame edge.
constexpr double kMargin = 24.0;
// How near (screen pixels) a click must be to pick a node.
constexpr double kHit = 5.0;
// Half a node square's side, screen pixels.
constexpr double kNode = 3.0;

QList<QPointF> pointsFrom(const ::rust::Vec<double>& flat)
{
    QList<QPointF> points;
    for (size_t i = 0; i + 1 < flat.size(); i += 2) {
        points.append(QPointF(flat[i], flat[i + 1]));
    }
    return points;
}

} // namespace

VpCanvas::VpCanvas(QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("vpCanvas"));
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_OpaquePaintEvent);
    setMinimumSize(320, 240);
    updateCursor();
}

void VpCanvas::reload(const QImage& backdrop)
{
    backdrop_ = backdrop;
    const ::rust::Vec<int32_t> dims = vp_geometry();
    if (dims.size() == 2) {
        docWidth_ = dims[0];
        docHeight_ = dims[1];
    }
    placed_ = width() > 0 && height() > 0;
    refreshPlanes();
    fitInView();
}

void VpCanvas::refreshPlanes()
{
    planes_.clear();
    const int count = vp_plane_count();
    for (int i = 0; i < count; ++i) {
        Plane plane;
        plane.corners = pointsFrom(vp_plane_corners(i));
        plane.edgeNodes = pointsFrom(vp_plane_edge_nodes(i));
        plane.validity = vp_plane_validity(i);
        const ::rust::Vec<double> grid = vp_plane_grid_lines(i);
        for (size_t k = 0; k + 3 < grid.size(); k += 4) {
            plane.grid.append(QLineF(grid[k], grid[k + 1], grid[k + 2], grid[k + 3]));
        }
        planes_.append(plane);
    }
    active_ = vp_active();
    update();
}

void VpCanvas::setMode(Mode mode)
{
    mode_ = mode;
    if (mode != Mode::CreatePlane) {
        cancelPending();
    }
    updateCursor();
}

void VpCanvas::setShowEdges(bool on)
{
    showEdges_ = on;
    update();
}

void VpCanvas::setZoom(double zoom)
{
    zoomAbout(zoom, QPointF(width() / 2.0, height() / 2.0));
}

void VpCanvas::zoomBy(double factor)
{
    setZoom(zoom_ * factor);
}

void VpCanvas::fitInView()
{
    if (docWidth_ <= 0 || docHeight_ <= 0) {
        return;
    }
    const double fit = std::min({(width() - 2 * kMargin) / docWidth_,
                                 (height() - 2 * kMargin) / docHeight_, 1.0});
    zoom_ = std::clamp(fit, kMinZoom, kMaxZoom);
    offset_ = QPointF((width() - docWidth_ * zoom_) / 2.0, (height() - docHeight_ * zoom_) / 2.0);
    viewChanged();
}

QPointF VpCanvas::toDocument(const QPointF& widget) const
{
    return (widget - offset_) / zoom_;
}

QPointF VpCanvas::toWidget(const QPointF& document) const
{
    return offset_ + document * zoom_;
}

void VpCanvas::cancelPending()
{
    pending_.clear();
    update();
}

QColor VpCanvas::validityColor(int validity)
{
    switch (validity) {
    case 1:
        return QColor(255, 0, 0);
    case 2:
        return QColor(255, 210, 0);
    default:
        return QColor(60, 110, 255);
    }
}

void VpCanvas::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), QColor(0xa8, 0xa8, 0xa8));
    if (docWidth_ <= 0) {
        return;
    }
    const QRectF target(toWidget(QPointF(0, 0)), QSizeF(docWidth_ * zoom_, docHeight_ * zoom_));
    painter.setRenderHint(QPainter::SmoothPixmapTransform, zoom_ < 1.0);
    if (!backdrop_.isNull()) {
        painter.drawImage(target, backdrop_);
    }
    if (!showEdges_) {
        return;
    }

    painter.setRenderHint(QPainter::Antialiasing, true);
    auto pen = [](const QColor& color, double width) {
        QPen p(color, width);
        p.setCosmetic(true);
        return p;
    };
    auto widgetPolygon = [this](const QList<QPointF>& points) {
        QPolygonF polygon;
        for (const QPointF& p : points) {
            polygon << toWidget(p);
        }
        return polygon;
    };
    for (int i = 0; i < planes_.size(); ++i) {
        const Plane& plane = planes_.at(i);
        const QColor color = validityColor(plane.validity);
        QColor gridColor = color;
        gridColor.setAlpha(150);
        painter.setPen(pen(gridColor, 1.0));
        for (const QLineF& line : plane.grid) {
            painter.drawLine(QLineF(toWidget(line.p1()), toWidget(line.p2())));
        }
        painter.setPen(pen(color, i == active_ ? 2.0 : 1.0));
        painter.setBrush(Qt::NoBrush);
        painter.drawPolygon(widgetPolygon(plane.corners));
    }

    // Create Plane: the corners so far, and the next edge to the pointer.
    if (!pending_.isEmpty()) {
        QList<QPointF> chain = pending_;
        if (hovering_) {
            chain.append(toDocument(hover_));
        }
        int validity = 0;
        if (chain.size() == 4) {
            std::vector<double> flat;
            for (const QPointF& p : chain) {
                flat.push_back(p.x());
                flat.push_back(p.y());
            }
            validity = vp_quad_validity(::rust::Slice<const double>(flat.data(), flat.size()));
        }
        painter.setPen(pen(validityColor(validity), 1.0));
        const QPolygonF polygon = widgetPolygon(chain);
        if (chain.size() == 4) {
            painter.drawPolygon(polygon);
        } else {
            painter.drawPolyline(polygon);
        }
    }

    painter.setRenderHint(QPainter::Antialiasing, false);
    auto node = [&painter](const QPointF& at) {
        painter.setPen(QPen(Qt::black, 1.0));
        painter.setBrush(Qt::white);
        painter.drawRect(QRectF(at - QPointF(kNode, kNode), QSizeF(2 * kNode, 2 * kNode)));
    };
    if (active_ >= 0 && active_ < planes_.size()) {
        for (const QPointF& p : planes_.at(active_).corners) {
            node(toWidget(p));
        }
        for (const QPointF& p : planes_.at(active_).edgeNodes) {
            node(toWidget(p));
        }
    }
    for (const QPointF& p : pending_) {
        node(toWidget(p));
    }
}

void VpCanvas::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    if (!placed_ && docWidth_ > 0) {
        placed_ = true;
        fitInView();
        return;
    }
    viewChanged();
}

bool VpCanvas::beginPlaneGesture(const QPointF& pos, Qt::KeyboardModifiers modifiers)
{
    const QPointF doc = toDocument(pos);
    const ::rust::Vec<int32_t> hit = vp_hit(doc.x(), doc.y(), kHit / zoom_);
    const bool ctrl = modifiers & Qt::ControlModifier;
    const bool alt = modifiers & Qt::AltModifier;
    if (hit.size() != 3) {
        if (ctrl) {
            return false;
        }
        vp_set_active(-1);
        refreshPlanes();
        if (planesChanged) {
            planesChanged();
        }
        return false;
    }
    const int plane = hit[0];
    const int part = hit[1];
    const int nodeIndex = hit[2];
    // Ctrl on an edge node tears off; Ctrl elsewhere walks overlapping planes.
    if (ctrl && part != 1) {
        vp_cycle(doc.x(), doc.y());
        refreshPlanes();
        if (planesChanged) {
            planesChanged();
        }
        return false;
    }
    const int mode = ctrl ? 1 : (alt && part == 1 ? 2 : 0);
    const bool started = vp_drag_begin(plane, part, nodeIndex, mode, doc.x(), doc.y());
    refreshPlanes();
    if (planesChanged) {
        planesChanged();
    }
    return started;
}

void VpCanvas::addCorner(const QPointF& document)
{
    pending_.append(document);
    if (pending_.size() == 4) {
        std::vector<double> flat;
        for (const QPointF& p : pending_) {
            flat.push_back(p.x());
            flat.push_back(p.y());
        }
        pending_.clear();
        vp_create(::rust::Slice<const double>(flat.data(), flat.size()));
        refreshPlanes();
        if (planesChanged) {
            planesChanged();
        }
        if (planeCreated) {
            planeCreated();
        }
    }
    update();
}

void VpCanvas::mousePressEvent(QMouseEvent* event)
{
    setFocus(Qt::MouseFocusReason);
    const QPointF pos = event->position();
    const Mode mode = event->button() == Qt::MiddleButton ? Mode::Hand : effectiveMode();
    if (mode == Mode::Hand) {
        drag_ = Drag::Pan;
        pressWidget_ = pos;
        panOffset_ = offset_;
        setCursor(Qt::ClosedHandCursor);
        return;
    }
    if (event->button() != Qt::LeftButton) {
        return;
    }
    switch (mode) {
    case Mode::Zoom:
        zoomAbout(zoom_ * ((event->modifiers() & Qt::AltModifier) ? 0.5 : 2.0), pos);
        return;
    case Mode::EditPlane:
        if (beginPlaneGesture(pos, event->modifiers())) {
            drag_ = Drag::Plane;
        }
        return;
    case Mode::CreatePlane: {
        // Existing nodes still drag while no corner is pending.
        if (pending_.isEmpty()) {
            const QPointF doc = toDocument(pos);
            const ::rust::Vec<int32_t> hit = vp_hit(doc.x(), doc.y(), kHit / zoom_);
            if (hit.size() == 3 && hit[1] != 2) {
                if (beginPlaneGesture(pos, event->modifiers())) {
                    drag_ = Drag::Plane;
                }
                return;
            }
        }
        addCorner(toDocument(pos));
        return;
    }
    case Mode::Hand:
        return;
    }
}

void VpCanvas::mouseMoveEvent(QMouseEvent* event)
{
    const QPointF pos = event->position();
    hover_ = pos;
    hovering_ = true;
    switch (drag_) {
    case Drag::Pan:
        offset_ = panOffset_ + (pos - pressWidget_);
        viewChanged();
        return;
    case Drag::Plane: {
        const QPointF doc = toDocument(pos);
        vp_drag_to(doc.x(), doc.y());
        refreshPlanes();
        if (planesChanged) {
            planesChanged();
        }
        return;
    }
    case Drag::None:
        if (!pending_.isEmpty()) {
            update();
        }
        return;
    }
}

void VpCanvas::mouseReleaseEvent(QMouseEvent*)
{
    if (drag_ == Drag::Plane) {
        vp_drag_end();
        refreshPlanes();
        if (planesChanged) {
            planesChanged();
        }
    }
    drag_ = Drag::None;
    updateCursor();
}

void VpCanvas::wheelEvent(QWheelEvent* event)
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

void VpCanvas::keyPressEvent(QKeyEvent* event)
{
    switch (event->key()) {
    case Qt::Key_Space:
        if (!event->isAutoRepeat()) {
            spaceHeld_ = true;
            updateCursor();
        }
        return;
    case Qt::Key_X:
        // Hold X to look closer about the pointer.
        if (!event->isAutoRepeat() && heldZoom_ == 0.0 && event->modifiers() == Qt::NoModifier) {
            heldZoom_ = zoom_;
            const QPointF at = hovering_ ? hover_ : QPointF(width() / 2.0, height() / 2.0);
            zoomAbout(zoom_ * 2.0, at);
        }
        return;
    case Qt::Key_Backspace:
        if (!pending_.isEmpty()) {
            pending_.removeLast();
            update();
            return;
        }
        [[fallthrough]];
    case Qt::Key_Delete:
        if (vp_delete_active()) {
            refreshPlanes();
            if (planesChanged) {
                planesChanged();
            }
        }
        return;
    case Qt::Key_Escape:
        if (!pending_.isEmpty()) {
            cancelPending();
            return;
        }
        break;
    default:
        break;
    }
    QWidget::keyPressEvent(event);
}

void VpCanvas::keyReleaseEvent(QKeyEvent* event)
{
    if (event->isAutoRepeat()) {
        QWidget::keyReleaseEvent(event);
        return;
    }
    if (event->key() == Qt::Key_Space) {
        spaceHeld_ = false;
        updateCursor();
        return;
    }
    if (event->key() == Qt::Key_X && heldZoom_ != 0.0) {
        const double back = heldZoom_;
        heldZoom_ = 0.0;
        const QPointF at = hovering_ ? hover_ : QPointF(width() / 2.0, height() / 2.0);
        zoomAbout(back, at);
        return;
    }
    QWidget::keyReleaseEvent(event);
}

VpCanvas::Mode VpCanvas::effectiveMode() const
{
    return spaceHeld_ ? Mode::Hand : mode_;
}

void VpCanvas::updateCursor()
{
    switch (effectiveMode()) {
    case Mode::Hand:
        setCursor(Qt::OpenHandCursor);
        break;
    case Mode::EditPlane:
        setCursor(Qt::ArrowCursor);
        break;
    case Mode::Zoom:
    case Mode::CreatePlane:
        setCursor(Qt::CrossCursor);
        break;
    }
}

void VpCanvas::zoomAbout(double zoom, const QPointF& anchor)
{
    const double next = std::clamp(zoom, kMinZoom, kMaxZoom);
    const QPointF doc = toDocument(anchor);
    zoom_ = next;
    offset_ = anchor - doc * zoom_;
    viewChanged();
}

void VpCanvas::viewChanged()
{
    clampOffset();
    update();
    if (zoomChanged) {
        zoomChanged();
    }
}

void VpCanvas::clampOffset()
{
    const double w = docWidth_ * zoom_;
    const double h = docHeight_ * zoom_;
    auto clampAxis = [](double offset, double extent, double frame) {
        if (extent <= frame - 2 * kMargin) {
            return (frame - extent) / 2.0;
        }
        return std::clamp(offset, frame - extent - kMargin, kMargin);
    };
    offset_ = QPointF(clampAxis(offset_.x(), w, width()), clampAxis(offset_.y(), h, height()));
}

} // namespace pictura

#include "moc_vp_canvas.cpp"
