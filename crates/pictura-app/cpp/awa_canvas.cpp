#include "awa_canvas.h"

#include <QtGui/QAction>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QTransform>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QMenu>

#include <algorithm>
#include <cmath>
#include <vector>

#include "pictura_app/src/cxxqt_object/filter_tools/adaptive_wide_angle.cxxqt.h"

namespace pictura {

namespace {

constexpr double kMinZoom = 0.0625;
constexpr double kMaxZoom = 16.0;
// How much ground stays between a panned picture and the frame edge.
constexpr double kMargin = 24.0;
// How near (screen pixels) a click must be to pick a constraint or handle.
constexpr double kHit = 5.0;

// The Zoom tool menu's levels in CS6's groups; 0 separates.
constexpr double kZoomLevels[] = {0.0625, 0.125, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0,
                                  1.0,    0,     2.0,  3.0,       4.0, 6.0,       8.0,
                                  10.0,   12.0,  16.0};

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

// Flattened `x, y` pairs, a NaN pair ending each polyline.
QPainterPath pathFrom(const ::rust::Vec<double>& points)
{
    QPainterPath path;
    bool start = true;
    for (size_t i = 0; i + 1 < points.size(); i += 2) {
        if (std::isnan(points[i])) {
            start = true;
            continue;
        }
        const QPointF p(points[i], points[i + 1]);
        if (start) {
            path.moveTo(p);
            start = false;
        } else {
            path.lineTo(p);
        }
    }
    return path;
}

double segmentDistance(const QPointF& p, const QPointF& a, const QPointF& b)
{
    const QPointF d = b - a;
    const double len2 = QPointF::dotProduct(d, d);
    const double t = len2 > 0.0 ? std::clamp(QPointF::dotProduct(p - a, d) / len2, 0.0, 1.0) : 0.0;
    const QPointF q = a + d * t - p;
    return std::hypot(q.x(), q.y());
}

double distance(const QPointF& a, const QPointF& b)
{
    return std::hypot(a.x() - b.x(), a.y() - b.y());
}

} // namespace

AwaCanvas::AwaCanvas(QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("awaCanvas"));
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_OpaquePaintEvent);
    setMinimumSize(320, 240);
    updateCursor();
}

void AwaCanvas::reload()
{
    const ::rust::Vec<int32_t> dims = awa_geometry();
    if (dims.size() == 2) {
        layerWidth_ = dims[0];
        layerHeight_ = dims[1];
    }
    // CS6 opens the workspace at 100%, centred.
    zoom_ = 1.0;
    offset_ = QPointF((width() - layerWidth_) / 2.0, (height() - layerHeight_) / 2.0);
    placed_ = width() > 0 && height() > 0;
    viewChanged();
}

void AwaCanvas::refreshAll()
{
    const double dpr = devicePixelRatioF();
    const int w = static_cast<int>(std::ceil(width() * dpr));
    const int h = static_cast<int>(std::ceil(height() * dpr));
    const QPointF origin = toDisplay(QPointF(0, 0));
    view_ = awa_render(origin.x(), origin.y(), zoom_ * dpr, w, h, preview_);
    view_.setDevicePixelRatio(dpr);
    readShapes();
    mesh_ = showMesh_ ? pathFrom(awa_mesh_lines(preview_)) : QPainterPath();
    update();
}

void AwaCanvas::readShapes()
{
    shapes_.clear();
    const int count = awa_constraint_count();
    for (int i = 0; i < count; ++i) {
        Shape shape;
        shape.state = awa_constraint_state(i);
        shape.polygon = awa_constraint_polygon(i);
        shape.path = pathFrom(awa_constraint_paths(i, preview_));
        const ::rust::Vec<double> points = awa_constraint_points(i, preview_);
        for (size_t k = 0; k + 1 < points.size(); k += 2) {
            shape.points.append(QPointF(points[k], points[k + 1]));
        }
        shapes_.append(shape);
    }
    if (selected_ >= count) {
        selected_ = -1;
    }
}

void AwaCanvas::setMode(Mode mode)
{
    mode_ = mode;
    if (mode != Mode::Polygon) {
        cancelPolygon();
    }
    updateCursor();
    update();
}

void AwaCanvas::setPreview(bool on)
{
    preview_ = on;
    cancelPolygon();
    refreshAll();
}

void AwaCanvas::setShowConstraints(bool on)
{
    showConstraints_ = on;
    update();
}

void AwaCanvas::setShowMesh(bool on)
{
    showMesh_ = on;
    refreshAll();
}

void AwaCanvas::setZoom(double zoom)
{
    zoomAbout(zoom, QPointF(width() / 2.0, height() / 2.0));
}

void AwaCanvas::zoomBy(double factor)
{
    setZoom(zoom_ * factor);
}

void AwaCanvas::fitInView()
{
    fit(true);
}

void AwaCanvas::fitOnScreen()
{
    fit(false);
}

void AwaCanvas::fit(bool capAtActualPixels)
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

QList<QAction*> AwaCanvas::zoomMenuActions(QObject* owner)
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

void AwaCanvas::showZoomMenu(const QPoint& globalPos)
{
    QMenu menu(this);
    menu.setObjectName(QStringLiteral("awaZoomMenu"));
    menu.addActions(zoomMenuActions(&menu));
    menu.exec(globalPos);
}

QPointF AwaCanvas::toDisplay(const QPointF& widget) const
{
    return (widget - offset_) / zoom_;
}

QPointF AwaCanvas::toWidget(const QPointF& display) const
{
    return offset_ + display * zoom_;
}

void AwaCanvas::select(int index)
{
    selected_ = index >= 0 && index < shapes_.size() ? index : -1;
    update();
}

int AwaCanvas::constraintAt(const QPointF& pos) const
{
    if (!showConstraints_) {
        return -1;
    }
    for (int i = static_cast<int>(shapes_.size()) - 1; i >= 0; --i) {
        const QPainterPath& path = shapes_.at(i).path;
        for (int k = 1; k < path.elementCount(); ++k) {
            const QPainterPath::Element b = path.elementAt(k);
            if (b.isMoveTo()) {
                continue;
            }
            const QPainterPath::Element a = path.elementAt(k - 1);
            if (segmentDistance(pos, toWidget(QPointF(a.x, a.y)), toWidget(QPointF(b.x, b.y))) <=
                kHit) {
                return i;
            }
        }
    }
    return -1;
}

int AwaCanvas::handleAt(const QPointF& pos) const
{
    if (selected_ < 0 || selected_ >= shapes_.size()) {
        return -1;
    }
    const QList<QPointF>& points = shapes_.at(selected_).points;
    for (int k = 0; k < points.size(); ++k) {
        if (distance(toWidget(points.at(k)), pos) <= kHit + 1.0) {
            return k;
        }
    }
    return -1;
}

QMenu* AwaCanvas::constraintMenu(int index, QWidget* owner)
{
    auto* menu = new QMenu(owner);
    menu->setObjectName(QStringLiteral("awaConstraintMenu"));
    if (!awa_constraint_polygon(index)) {
        const struct {
            const char* label;
            int code;
        } orientations[] = {{"Unfixed", 0}, {"Horizontal", 1}, {"Vertical", 2}, {"Arbitrary", 3}};
        for (const auto& o : orientations) {
            QAction* action = menu->addAction(QString::fromLatin1(o.label));
            action->setObjectName(QStringLiteral("awaOrient") + QString::fromLatin1(o.label));
            connect(action, &QAction::triggered, this, [this, index, code = o.code] {
                awa_set_orientation(index, code);
                refreshAll();
            });
        }
        menu->addSeparator();
    }
    QAction* remove = menu->addAction(QStringLiteral("Delete"));
    remove->setObjectName(QStringLiteral("awaDelete"));
    connect(remove, &QAction::triggered, this, [this, index] {
        awa_remove(index);
        selected_ = -1;
        refreshAll();
    });
    return menu;
}

QColor AwaCanvas::stateColor(int state)
{
    switch (state) {
    case 1:
        return QColor(255, 255, 0); // horizontal
    case 2:
        return QColor(255, 0, 255); // vertical
    case 3:
        return QColor(0, 255, 0); // fixed
    case 4:
        return QColor(255, 0, 0); // invalid
    default:
        return QColor(0, 255, 255); // unfixed
    }
}

QColor AwaCanvas::constraintColor(int index) const
{
    return index >= 0 && index < shapes_.size() ? stateColor(shapes_.at(index).state) : QColor();
}

void AwaCanvas::removeLastCorner()
{
    if (!corners_.isEmpty()) {
        corners_.removeLast();
        cornerDisplay_.removeLast();
        update();
    }
}

void AwaCanvas::cancelPolygon()
{
    corners_.clear();
    cornerDisplay_.clear();
    update();
}

QPainterPath AwaCanvas::meshPathForTest() const
{
    return QTransform::fromTranslate(offset_.x(), offset_.y()).scale(zoom_, zoom_).map(mesh_);
}

bool AwaCanvas::layerAt(const QPointF& pos, QPointF* layer) const
{
    const QPointF d = toDisplay(pos);
    const ::rust::Vec<double> p = awa_to_layer(d.x(), d.y(), preview_);
    if (p.size() != 2) {
        return false;
    }
    *layer = QPointF(p[0], p[1]);
    return true;
}

QPainterPath AwaCanvas::arcPath(const QPointF& a, const QPointF& b) const
{
    return pathFrom(awa_arc(a.x(), a.y(), b.x(), b.y(), preview_));
}

void AwaCanvas::addCorner(const QPointF& pos)
{
    if (corners_.size() >= 3 && distance(toWidget(cornerDisplay_.first()), pos) <= kHit + 2.0) {
        std::vector<double> flat;
        for (const QPointF& c : corners_) {
            flat.push_back(c.x());
            flat.push_back(c.y());
        }
        const int index = awa_add_polygon(::rust::Slice<const double>(flat.data(), flat.size()));
        cancelPolygon();
        refreshAll();
        select(index);
        return;
    }
    QPointF layer;
    if (!layerAt(pos, &layer)) {
        return;
    }
    corners_.append(layer);
    cornerDisplay_.append(toDisplay(pos));
    update();
}

void AwaCanvas::paintEvent(QPaintEvent*)
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

    painter.save();
    painter.translate(offset_);
    painter.scale(zoom_, zoom_);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setBrush(Qt::NoBrush);
    auto pen = [](const QColor& color, double width) {
        QPen p(color, width);
        p.setCosmetic(true);
        return p;
    };
    if (showMesh_) {
        painter.setPen(pen(QColor(128, 128, 128), 1.0));
        painter.drawPath(mesh_);
    }
    if (showConstraints_) {
        for (int i = 0; i < shapes_.size(); ++i) {
            painter.setPen(pen(stateColor(shapes_.at(i).state), i == selected_ ? 2.0 : 1.0));
            painter.drawPath(shapes_.at(i).path);
        }
    }
    const QColor pending = stateColor(0);
    if (drag_ == Drag::Line && lineEndValid_) {
        painter.setPen(pen(pending, 1.0));
        painter.drawPath(arcPath(lineStart_, lineEnd_));
    }
    if (!corners_.isEmpty()) {
        painter.setPen(pen(pending, 1.0));
        for (int k = 1; k < corners_.size(); ++k) {
            painter.drawPath(arcPath(corners_.at(k - 1), corners_.at(k)));
        }
        QPointF layer;
        if (hovering_ && layerAt(hover_, &layer)) {
            painter.drawPath(arcPath(corners_.last(), layer));
        }
    }
    painter.restore();

    // Handles: the selected constraint's points and the pending corners.
    painter.setRenderHint(QPainter::Antialiasing, false);
    auto handle = [&painter](const QPointF& at) {
        painter.setPen(QPen(Qt::black, 1.0));
        painter.setBrush(Qt::white);
        painter.drawRect(QRectF(at - QPointF(3, 3), QSizeF(6, 6)));
    };
    if (showConstraints_ && selected_ >= 0 && selected_ < shapes_.size()) {
        for (const QPointF& p : shapes_.at(selected_).points) {
            handle(toWidget(p));
        }
    }
    for (const QPointF& p : cornerDisplay_) {
        handle(toWidget(p));
    }
}

void AwaCanvas::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    if (!placed_ && layerWidth_ > 0) {
        reload();
        return;
    }
    viewChanged();
}

void AwaCanvas::mousePressEvent(QMouseEvent* event)
{
    setFocus(Qt::MouseFocusReason);
    const QPointF pos = event->position();
    const Mode mode = event->button() == Qt::MiddleButton ? Mode::Hand : effectiveMode();
    if (event->button() == Qt::RightButton) {
        if (mode == Mode::Zoom) {
            showZoomMenu(event->globalPosition().toPoint());
            return;
        }
        const int hit = constraintAt(pos);
        if (hit >= 0) {
            select(hit);
            QMenu* menu = constraintMenu(hit, this);
            menu->exec(event->globalPosition().toPoint());
            delete menu;
        }
        return;
    }
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
    case Mode::Move: {
        drag_ = Drag::Move;
        pressWidget_ = pos;
        moveStart_.clear();
        for (double v : awa_offset()) {
            moveStart_.append(v);
        }
        return;
    }
    case Mode::Constraint:
    case Mode::Polygon:
    case Mode::Hand:
        break;
    }
    if (corners_.isEmpty()) {
        if (const int h = handleAt(pos); h >= 0) {
            drag_ = Drag::Handle;
            handle_ = h;
            return;
        }
        if (const int hit = constraintAt(pos); hit >= 0) {
            select(hit);
            return;
        }
    }
    if (mode == Mode::Polygon) {
        select(-1);
        addCorner(pos);
        return;
    }
    QPointF layer;
    select(-1);
    if (!layerAt(pos, &layer)) {
        return;
    }
    drag_ = Drag::Line;
    pressWidget_ = pos;
    lineStart_ = layer;
    lineEnd_ = layer;
    lineEndValid_ = false;
}

void AwaCanvas::mouseMoveEvent(QMouseEvent* event)
{
    const QPointF pos = event->position();
    hover_ = pos;
    hovering_ = true;
    if (hoverMoved) {
        hoverMoved(toDisplay(pos));
    }
    switch (drag_) {
    case Drag::Pan:
        offset_ = panOffset_ + (pos - pressWidget_);
        viewChanged();
        return;
    case Drag::Line:
        lineEndValid_ = layerAt(pos, &lineEnd_);
        update();
        return;
    case Drag::Handle: {
        QPointF layer;
        if (layerAt(pos, &layer)) {
            awa_move_point(selected_, handle_, layer.x(), layer.y());
            refreshAll();
        }
        return;
    }
    case Drag::Move: {
        if (moveStart_.size() == 2) {
            const QPointF d = (pos - pressWidget_) / zoom_;
            awa_set_offset(moveStart_.at(0) + d.x(), moveStart_.at(1) + d.y());
            refreshAll();
        }
        return;
    }
    case Drag::None:
        if (!corners_.isEmpty()) {
            update();
        }
        return;
    }
}

void AwaCanvas::mouseReleaseEvent(QMouseEvent* event)
{
    const Drag drag = drag_;
    drag_ = Drag::None;
    if (drag == Drag::Line) {
        const QPointF pos = event->position();
        lineEndValid_ = layerAt(pos, &lineEnd_);
        if (lineEndValid_ && distance(pos, pressWidget_) > 3.0) {
            int orientation = 0;
            if (event->modifiers() & Qt::ShiftModifier) {
                const QPointF d = pos - pressWidget_;
                orientation = std::abs(d.x()) >= std::abs(d.y()) ? 1 : 2;
            }
            const int index = awa_add_line(lineStart_.x(), lineStart_.y(), lineEnd_.x(),
                                           lineEnd_.y(), orientation);
            refreshAll();
            select(index);
        }
        update();
    } else if (drag == Drag::Move && offsetChanged) {
        offsetChanged();
    }
    updateCursor();
}

void AwaCanvas::wheelEvent(QWheelEvent* event)
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

void AwaCanvas::keyPressEvent(QKeyEvent* event)
{
    switch (event->key()) {
    case Qt::Key_Space:
        if (!event->isAutoRepeat()) {
            spaceHeld_ = true;
            updateCursor();
        }
        return;
    case Qt::Key_Delete:
    case Qt::Key_Backspace:
        if (selected_ >= 0) {
            awa_remove(selected_);
            selected_ = -1;
            refreshAll();
        }
        return;
    case Qt::Key_E:
        if (!corners_.isEmpty()) {
            removeLastCorner();
            return;
        }
        break;
    case Qt::Key_Escape:
        if (!corners_.isEmpty()) {
            cancelPolygon();
            return;
        }
        break;
    default:
        break;
    }
    QWidget::keyPressEvent(event);
}

void AwaCanvas::keyReleaseEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Space && !event->isAutoRepeat()) {
        spaceHeld_ = false;
        updateCursor();
        return;
    }
    QWidget::keyReleaseEvent(event);
}

AwaCanvas::Mode AwaCanvas::effectiveMode() const
{
    return spaceHeld_ ? Mode::Hand : mode_;
}

void AwaCanvas::updateCursor()
{
    switch (effectiveMode()) {
    case Mode::Hand:
        setCursor(Qt::OpenHandCursor);
        break;
    case Mode::Move:
        setCursor(Qt::SizeAllCursor);
        break;
    case Mode::Zoom:
    case Mode::Constraint:
    case Mode::Polygon:
        setCursor(Qt::CrossCursor);
        break;
    }
}

void AwaCanvas::zoomAbout(double zoom, const QPointF& anchor)
{
    const double next = std::clamp(zoom, kMinZoom, kMaxZoom);
    const QPointF display = toDisplay(anchor);
    zoom_ = next;
    offset_ = anchor - display * zoom_;
    viewChanged();
}

void AwaCanvas::viewChanged()
{
    clampOffset();
    refreshAll();
    if (zoomChanged) {
        zoomChanged();
    }
}

void AwaCanvas::clampOffset()
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

AwaDetail::AwaDetail(QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("awaDetail"));
    setFixedSize(256, 192);
}

void AwaDetail::showAt(const QPointF& display, bool preview)
{
    const double dpr = devicePixelRatioF();
    image_ = awa_render(display.x() - width() / 2.0, display.y() - height() / 2.0, dpr,
                        static_cast<int>(std::ceil(width() * dpr)),
                        static_cast<int>(std::ceil(height() * dpr)), preview);
    image_.setDevicePixelRatio(dpr);
    update();
}

void AwaDetail::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), checkerBrush());
    if (!image_.isNull()) {
        painter.drawImage(QPointF(0, 0), image_);
    }
    painter.setPen(palette().color(QPalette::Mid));
    painter.drawRect(rect().adjusted(0, 0, -1, -1));
}

} // namespace pictura

#include "moc_awa_canvas.cpp"
