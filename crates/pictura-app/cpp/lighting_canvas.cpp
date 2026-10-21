#include "lighting_canvas.h"

#include <QtCore/QLineF>
#include <QtCore/QtMath>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QRadialGradient>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

const double kMargin = 24.0;
const double kHandle = 4.5;
const double kRing = 13.0;
const double kGrab = 7.0;
// An Infinite light's disc is a widget, not a footprint: its size is fixed.
const double kSunRadius = 70.0;
const QColor kPasteboard(0x28, 0x28, 0x28);
const QColor kPointRing(0x4b, 0xe0, 0x4b);

QPointF unit(double degrees)
{
    const double r = qDegreesToRadians(degrees);
    return {std::cos(r), std::sin(r)};
}

double dot(const QPointF& a, const QPointF& b)
{
    return a.x() * b.x() + a.y() * b.y();
}

double length(const QPointF& v)
{
    return std::hypot(v.x(), v.y());
}

double wrapDegrees(double degrees)
{
    const double wrapped = std::fmod(degrees, 360.0);
    return wrapped < 0.0 ? wrapped + 360.0 : wrapped;
}

// The light's outline drawn white over a soft dark edge, so it reads on any
// picture, as CS6's does.
void stroke(QPainter& painter, const QPainterPath& path, const QColor& color)
{
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(0, 0, 0, 110), 3.0));
    painter.drawPath(path);
    painter.setPen(QPen(color, 1.3));
    painter.drawPath(path);
}

void drawHandle(QPainter& painter, const QPointF& at)
{
    painter.setPen(QPen(QColor(0, 0, 0, 140), 1.0));
    painter.setBrush(Qt::white);
    painter.drawEllipse(at, kHandle, kHandle);
}

// Normalised distance from a Spot's hotspot ellipse (1 on its edge), for a
// point at `local` in the Spot's frame (x along the aim).
double hotspotDistance(const QPointF& local, const HotspotShape& shape, double a, double b)
{
    const double ia = std::max(shape.major * a, 1e-3);
    const double ib = std::max(shape.minor * b, 1e-3);
    const double u = (local.x() - shape.offset * a) / ia;
    const double v = local.y() / ib;
    return std::sqrt(u * u + v * v);
}

} // namespace

LightingCanvas::LightingCanvas(LightingRig* rig, QWidget* parent) : QWidget(parent), rig_(rig)
{
    setObjectName(QStringLiteral("lightingCanvas"));
    setMinimumSize(320, 240);
    setFocusPolicy(Qt::ClickFocus);
    setMouseTracking(false);
}

void LightingCanvas::setImage(const QImage& image)
{
    image_ = image;
    update();
}

void LightingCanvas::setAspect(const QSize& aspect)
{
    aspect_ = aspect;
    update();
}

void LightingCanvas::setSelected(int index)
{
    selected_ = index;
    update();
}

QRectF LightingCanvas::imageRect() const
{
    const QRectF room = QRectF(rect()).adjusted(kMargin, kMargin, -kMargin, -kMargin);
    if (aspect_.isEmpty() || room.isEmpty()) {
        return room;
    }
    const QSizeF fitted = QSizeF(aspect_).scaled(room.size(), Qt::KeepAspectRatio);
    return QRectF(room.center() - QPointF(fitted.width(), fitted.height()) / 2.0, fitted);
}

QPointF LightingCanvas::toWidget(const QPointF& fraction) const
{
    const QRectF area = imageRect();
    return {area.x() + fraction.x() * area.width(), area.y() + fraction.y() * area.height()};
}

QPointF LightingCanvas::toFraction(const QPointF& widget) const
{
    const QRectF area = imageRect();
    if (area.width() <= 0.0 || area.height() <= 0.0) {
        return {0.5, 0.5};
    }
    return {(widget.x() - area.x()) / area.width(), (widget.y() - area.y()) / area.height()};
}

double LightingCanvas::span() const
{
    const QRectF area = imageRect();
    return 0.5 * std::hypot(area.width(), area.height());
}

QPointF LightingCanvas::handlePosition(int index, Part part) const
{
    if (index < 0 || index >= rig_->lights.size()) {
        return {};
    }
    const LightSpec& light = rig_->lights.at(index);
    const QPointF c = toWidget(light.center);
    const QPointF dir = unit(light.angle);
    const QPointF across(-dir.y(), dir.x());
    switch (part) {
    case Part::Major:
        return c + dir * light.size * span();
    case Part::Minor:
        return c + across * light.width * span();
    case Part::Direction:
        return c + dir * kSunRadius * std::cos(qDegreesToRadians(light.elevation));
    case Part::Intensity:
        return c + QPointF(0.0, -kRing);
    default:
        return c;
    }
}

LightingCanvas::Part LightingCanvas::hitTest(const QPointF& pos, int* index) const
{
    *index = -1;
    const int count = int(rig_->lights.size());
    const bool hasSelected = selected_ >= 0 && selected_ < count;
    const double s = span();

    // The selected light's Intensity ring first, then any other light's centre
    // (a small, deliberate target), then the selected light's edges and body.
    if (hasSelected &&
        std::abs(QLineF(pos, toWidget(rig_->lights.at(selected_).center)).length() - kRing) <= 4.0) {
        *index = selected_;
        return Part::Intensity;
    }
    for (int i = count - 1; i >= 0; --i) {
        if (i != selected_ && QLineF(pos, toWidget(rig_->lights.at(i).center)).length() <= kRing + 3.0) {
            *index = i;
            return Part::Move;
        }
    }
    if (hasSelected) {
        const LightSpec& light = rig_->lights.at(selected_);
        const QPointF c = toWidget(light.center);
        const QPointF d = pos - c;
        const QPointF dir = unit(light.angle);
        const QPointF local(dot(d, dir), dot(d, QPointF(-dir.y(), dir.x())));
        *index = selected_;
        switch (light.kind) {
        case LightKind::Spot: {
            const double a = light.size * s;
            const double b = light.width * s;
            if (QLineF(pos, handlePosition(selected_, Part::Major)).length() <= kGrab ||
                QLineF(pos, c - dir * a).length() <= kGrab) {
                return Part::Major;
            }
            if (QLineF(pos, handlePosition(selected_, Part::Minor)).length() <= kGrab ||
                QLineF(pos, c - (handlePosition(selected_, Part::Minor) - c)).length() <= kGrab) {
                return Part::Minor;
            }
            const HotspotShape shape = spotHotspot(light.hotspot);
            const double reach = std::min(shape.major * a, shape.minor * b);
            if (reach > 4.0 && std::abs(hotspotDistance(local, shape, a, b) - 1.0) * reach <= 4.0) {
                return Part::Hotspot;
            }
            break;
        }
        case LightKind::Point:
            if (std::abs(length(d) - light.size * s) <= 5.0) {
                return Part::Radius;
            }
            break;
        case LightKind::Infinite:
            if (QLineF(pos, handlePosition(selected_, Part::Direction)).length() <= kGrab + 2.0) {
                return Part::Direction;
            }
            break;
        }
    }

    if (hasSelected) {
        const LightSpec& light = rig_->lights.at(selected_);
        const QPointF d = pos - toWidget(light.center);
        const QPointF dir = unit(light.angle);
        *index = selected_;
        switch (light.kind) {
        case LightKind::Spot: {
            const double a = std::max(light.size * s, 1.0);
            const double b = std::max(light.width * s, 1.0);
            const double u = dot(d, dir) / a;
            const double v = dot(d, QPointF(-dir.y(), dir.x())) / b;
            return u * u + v * v <= 1.0 ? Part::Move : Part::Rotate;
        }
        case LightKind::Point:
            if (length(d) <= std::max(light.size * s, kRing)) {
                return Part::Move;
            }
            break;
        case LightKind::Infinite:
            if (length(d) <= kSunRadius) {
                return Part::Move;
            }
            break;
        }
    }
    *index = -1;
    return Part::None;
}

void LightingCanvas::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), kPasteboard);
    const QRectF area = imageRect();
    if (!image_.isNull()) {
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawImage(area, image_);
    }
    painter.setRenderHint(QPainter::Antialiasing, true);
    const double s = span();

    for (int i = 0; i < rig_->lights.size(); ++i) {
        const LightSpec& light = rig_->lights.at(i);
        const bool active = i == selected_;
        const QPointF c = toWidget(light.center);
        painter.setOpacity(light.on ? (active ? 1.0 : 0.7) : 0.35);

        if (active) {
            switch (light.kind) {
            case LightKind::Spot: {
                const double a = light.size * s;
                const double b = light.width * s;
                const HotspotShape shape = spotHotspot(light.hotspot);
                painter.save();
                painter.translate(c);
                painter.rotate(light.angle);
                QPainterPath outer;
                outer.addEllipse(QPointF(0, 0), a, b);
                stroke(painter, outer, Qt::white);
                QPainterPath hot;
                hot.addEllipse(QPointF(shape.offset * a, 0), shape.major * a, shape.minor * b);
                stroke(painter, hot, Qt::white);
                painter.restore();
                const QPointF major = handlePosition(i, Part::Major) - c;
                const QPointF minor = handlePosition(i, Part::Minor) - c;
                for (const QPointF& at : {c + major, c - major, c + minor, c - minor}) {
                    drawHandle(painter, at);
                }
                break;
            }
            case LightKind::Point: {
                QPainterPath ring;
                ring.addEllipse(c, light.size * s, light.size * s);
                stroke(painter, ring, kPointRing);
                break;
            }
            case LightKind::Infinite: {
                painter.setPen(QPen(QColor(255, 255, 255, 60), 1.0));
                painter.setBrush(QColor(170, 170, 170, 105));
                painter.drawEllipse(c, kSunRadius, kSunRadius);
                const QPointF knob = handlePosition(i, Part::Direction);
                QPainterPath line;
                line.moveTo(c);
                line.lineTo(knob);
                stroke(painter, line, Qt::white);
                QRadialGradient shade(knob - QPointF(2.5, 2.5), 10.0);
                shade.setColorAt(0.0, Qt::white);
                shade.setColorAt(1.0, QColor(120, 120, 120));
                painter.setPen(QPen(QColor(0, 0, 0, 140), 1.0));
                painter.setBrush(shade);
                painter.drawEllipse(knob, 8.0, 8.0);
                break;
            }
            }
        }

        // The Intensity ring: a grey track with the light's share in white,
        // clockwise from the top.
        const QRectF ringRect(c - QPointF(kRing, kRing), QSizeF(2 * kRing, 2 * kRing));
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(0, 0, 0, 90), 4.5));
        painter.drawEllipse(ringRect);
        painter.setPen(QPen(QColor(255, 255, 255, 80), 2.5));
        painter.drawEllipse(ringRect);
        const double share = (std::clamp(light.intensity, -100.0, 100.0) + 100.0) / 200.0;
        painter.setPen(QPen(Qt::white, 2.5, Qt::SolidLine, Qt::FlatCap));
        painter.drawArc(ringRect, 90 * 16, -qRound(share * 360.0 * 16.0));
        // The centre handle.
        painter.setPen(QPen(QColor(0, 0, 0, 140), 3.0));
        painter.drawEllipse(c, 6.0, 6.0);
        painter.setPen(QPen(Qt::white, 1.5));
        painter.drawEllipse(c, 6.0, 6.0);
        painter.setBrush(Qt::white);
        painter.drawEllipse(c, 1.8, 1.8);
    }
}

void LightingCanvas::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        return;
    }
    int index = -1;
    const Part part = hitTest(event->position(), &index);
    if (part == Part::None) {
        return;
    }
    if (index != selected_) {
        setSelected(index);
        emit lightSelected(index);
    }
    if (part == Part::Move && (event->modifiers() & Qt::AltModifier) &&
        rig_->lights.size() < kMaxLights) {
        rig_->lights.append(rig_->lights.at(selected_));
        setSelected(int(rig_->lights.size()) - 1);
        emit lightDuplicated(selected_);
    }
    dragging_ = part;
    grabOffset_ = toWidget(rig_->lights.at(selected_).center) - event->position();
}

void LightingCanvas::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ == Part::None || selected_ < 0 || selected_ >= rig_->lights.size()) {
        return;
    }
    drag(event->position());
    update();
    emit rigEdited();
}

void LightingCanvas::mouseReleaseEvent(QMouseEvent*)
{
    dragging_ = Part::None;
}

void LightingCanvas::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) {
        emit deleteRequested();
        return;
    }
    QWidget::keyPressEvent(event);
}

void LightingCanvas::drag(const QPointF& pos)
{
    LightSpec& light = rig_->lights[selected_];
    const double s = std::max(span(), 1.0);
    const QPointF d = pos - toWidget(light.center);
    const QPointF dir = unit(light.angle);
    const QPointF across(-dir.y(), dir.x());
    switch (dragging_) {
    case Part::Move: {
        const QPointF at = toFraction(pos + grabOffset_);
        light.center = QPointF(std::clamp(at.x(), 0.0, 1.0), std::clamp(at.y(), 0.0, 1.0));
        break;
    }
    case Part::Rotate:
        if (length(d) > 1.0) {
            light.angle = wrapDegrees(qRadiansToDegrees(std::atan2(d.y(), d.x())));
        }
        break;
    case Part::Major:
        light.size = std::max(0.02, std::abs(dot(d, dir)) / s);
        break;
    case Part::Minor:
        light.width = std::max(0.02, std::abs(dot(d, across)) / s);
        break;
    case Part::Hotspot: {
        // The hotspot whose ellipse passes nearest the cursor.
        const QPointF local(dot(d, dir), dot(d, across));
        double best = light.hotspot;
        double bestError = 1e9;
        for (int h = -100; h <= 100; ++h) {
            const double error =
                std::abs(hotspotDistance(local, spotHotspot(h), light.size * s, light.width * s) - 1.0);
            if (error < bestError) {
                bestError = error;
                best = h;
            }
        }
        light.hotspot = best;
        break;
    }
    case Part::Intensity: {
        if (length(d) < 2.0) {
            break;
        }
        const double turn = wrapDegrees(qRadiansToDegrees(std::atan2(d.x(), -d.y()))) / 360.0;
        double intensity = std::round(turn * 200.0 - 100.0);
        // Crossing the top would jump between the ends; hold the end instead.
        if (light.intensity > 50.0 && intensity < -50.0) {
            intensity = 100.0;
        } else if (light.intensity < -50.0 && intensity > 50.0) {
            intensity = -100.0;
        }
        light.intensity = intensity;
        break;
    }
    case Part::Radius:
        light.size = std::max(0.02, length(d) / s);
        break;
    case Part::Direction: {
        if (length(d) < 1.0) {
            break;
        }
        const double reach = std::min(length(d) / kSunRadius, 1.0);
        light.angle = wrapDegrees(qRadiansToDegrees(std::atan2(d.y(), d.x())));
        light.elevation = qRadiansToDegrees(std::acos(reach));
        break;
    }
    case Part::None:
        break;
    }
}

} // namespace pictura

#include "moc_lighting_canvas.cpp"
