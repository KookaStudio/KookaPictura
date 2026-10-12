#include "blur_gallery_canvas.h"

#include <QtCore/QLineF>
#include <QtCore/QtMath>
#include <QtGui/QFontMetrics>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygonF>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

const double kMargin = 24.0;
const double kPin = 5.5;
const double kDot = 3.5;
const double kRing = 13.0;
const double kRingSmall = 8.0;
const double kGrab = 5.0;
const double kMaxBlur = 500.0;
const QColor kPasteboard(0x28, 0x28, 0x28);
const QColor kMarker(0xe0, 0xe0, 0xe0);
const double kHandle = 3.0;
const double kFeatherDot = 4.5;
const double kMinIrisRadius = 0.005;
const double kMaxIrisRadius = 10.0;

// The superellipse exponent for a squareness, as `pictura_filters::iris_exponent`.
double irisExponent(double squareness)
{
    return 2.0 / (1.0 - 0.009 * squareness);
}

double dot(const QPointF& a, const QPointF& b)
{
    return a.x() * b.x() + a.y() * b.y();
}

// A pin's outline in widget pixels: its centre, the unit axes along and
// across its angle, and the semi-axes.
struct IrisFrame {
    QPointF centre;
    QPointF along;
    QPointF across;
    double a = 0.0;
    double b = 0.0;
    double exponent = 2.0;

    QPointF at(double u, double v) const { return centre + along * (u * a) + across * (v * b); }
    // The roundness diamond: the outline's corner on the upper-right diagonal.
    QPointF diamond() const
    {
        const double k = std::pow(2.0, -1.0 / exponent);
        return at(k, -k);
    }
};

} // namespace

BlurGalleryCanvas::BlurGalleryCanvas(QList<BlurGalleryPin>* pins, QWidget* parent)
    : QWidget(parent), pins_(pins)
{
    setObjectName(QStringLiteral("blurGalleryCanvas"));
    setMinimumSize(320, 240);
    setFocusPolicy(Qt::ClickFocus);
}

void BlurGalleryCanvas::setImage(const QImage& image)
{
    image_ = image;
    update();
}

void BlurGalleryCanvas::setIris(bool on)
{
    iris_ = on;
    update();
}

void BlurGalleryCanvas::setUiVisible(bool on)
{
    uiVisible_ = on;
    update();
}

void BlurGalleryCanvas::setSelected(int index)
{
    if (!pins_ || index < 0 || index >= pins_->size()) {
        return;
    }
    selected_ = index;
    update();
}

QSize BlurGalleryCanvas::sizeHint() const
{
    return {640, 480};
}

QRectF BlurGalleryCanvas::imageRect() const
{
    const QRectF room = QRectF(rect()).adjusted(kMargin, kMargin, -kMargin, -kMargin);
    if (room.isEmpty()) {
        return room;
    }
    const QSizeF aspect = image_.isNull() ? QSizeF(documentSize_) : QSizeF(image_.size());
    if (aspect.isEmpty()) {
        return room;
    }
    const QSizeF fitted = aspect.scaled(room.size(), Qt::KeepAspectRatio);
    return QRectF(room.center() - QPointF(fitted.width(), fitted.height()) / 2.0, fitted);
}

QPointF BlurGalleryCanvas::toWidget(const QPointF& fraction) const
{
    const QRectF area = imageRect();
    return {area.x() + fraction.x() * area.width(), area.y() + fraction.y() * area.height()};
}

QPointF BlurGalleryCanvas::toFraction(const QPointF& widget) const
{
    const QRectF area = imageRect();
    if (area.width() <= 0.0 || area.height() <= 0.0) {
        return {0.5, 0.5};
    }
    return {(widget.x() - area.x()) / area.width(), (widget.y() - area.y()) / area.height()};
}

namespace {

IrisFrame irisFrame(const BlurGalleryPin& pin, const QPointF& centre, double widthPx)
{
    const double radians = qDegreesToRadians(pin.angle);
    IrisFrame frame;
    frame.centre = centre;
    frame.along = {std::cos(radians), std::sin(radians)};
    frame.across = {-std::sin(radians), std::cos(radians)};
    frame.a = pin.radii.width() * widthPx;
    frame.b = pin.radii.height() * widthPx;
    frame.exponent = irisExponent(pin.squareness);
    return frame;
}

} // namespace

int BlurGalleryCanvas::hitPin(const QPointF& widget, bool* onRing) const
{
    *onRing = false;
    if (!pins_) {
        return -1;
    }
    for (int i = pins_->size() - 1; i >= 0; --i) {
        if (QLineF(widget, toWidget(pins_->at(i).pos)).length() <= kRingSmall + 2.0) {
            return i;
        }
    }
    if (selected_ >= 0 && selected_ < pins_->size()
        && std::abs(QLineF(widget, toWidget(pins_->at(selected_).pos)).length() - kRing) <= kGrab) {
        *onRing = true;
        return selected_;
    }
    return -1;
}

bool BlurGalleryCanvas::grabIrisHandle(const QPointF& widget)
{
    if (!iris_ || !pins_ || selected_ < 0 || selected_ >= pins_->size()) {
        return false;
    }
    const BlurGalleryPin& pin = pins_->at(selected_);
    const IrisFrame frame = irisFrame(pin, toWidget(pin.pos), imageRect().width());
    const auto near = [&](const QPointF& handle, double size) {
        return QLineF(widget, handle).length() <= size + kGrab;
    };
    if (near(frame.diamond(), kHandle)) {
        drag_ = Drag::Roundness;
        return true;
    }
    for (const double sign : {1.0, -1.0}) {
        for (int axis = 0; axis < 2; ++axis) {
            const QPointF unit = axis == 0 ? QPointF(sign, 0.0) : QPointF(0.0, sign);
            if (near(frame.at(unit.x() * pin.feather, unit.y() * pin.feather), kFeatherDot)) {
                drag_ = Drag::Feather;
            } else if (near(frame.at(unit.x(), unit.y()), kHandle)) {
                drag_ = Drag::Radius;
            } else {
                continue;
            }
            dragAxis_ = axis;
            dragSign_ = sign;
            return true;
        }
    }
    return false;
}

void BlurGalleryCanvas::dragIris(const QPointF& widget, BlurGalleryPin& pin) const
{
    const double widthPx = imageRect().width();
    if (widthPx <= 0.0) {
        return;
    }
    const IrisFrame frame = irisFrame(pin, toWidget(pin.pos), widthPx);
    const QPointF offset = widget - frame.centre;
    if (drag_ == Drag::Radius) {
        // The grabbed handle follows the pointer: its distance sets that
        // semi-axis and its direction turns the outline.
        const double length = std::hypot(offset.x(), offset.y());
        if (length <= 0.0) {
            return;
        }
        const double radius = std::clamp(length / widthPx, kMinIrisRadius, kMaxIrisRadius);
        const QPointF axis = offset * (dragSign_ / length);
        if (dragAxis_ == 0) {
            pin.radii.setWidth(radius);
            pin.angle = qRadiansToDegrees(std::atan2(axis.y(), axis.x()));
        } else {
            pin.radii.setHeight(radius);
            pin.angle = qRadiansToDegrees(std::atan2(-axis.x(), axis.y()));
        }
    } else if (drag_ == Drag::Feather) {
        const double along = std::abs(dot(offset, dragAxis_ == 0 ? frame.along : frame.across));
        pin.feather = std::clamp(along / (dragAxis_ == 0 ? frame.a : frame.b), 0.0, 1.0);
    } else if (drag_ == Drag::Roundness) {
        // The diamond sits at the corner `2^(-1/p)` along both axes; invert
        // the exponent map from its mean normalized offset.
        const double u = std::abs(dot(offset, frame.along)) / std::max(frame.a, 1e-9);
        const double v = std::abs(dot(offset, frame.across)) / std::max(frame.b, 1e-9);
        const double corner = std::clamp((u + v) / 2.0, std::sqrt(0.5), 0.966);
        pin.squareness = std::clamp((1.0 + 2.0 * std::log2(corner)) / 0.009, 0.0, 100.0);
    }
}

void BlurGalleryCanvas::dragTo(const QPointF& widget)
{
    if (!pins_ || selected_ < 0 || selected_ >= pins_->size()) {
        return;
    }
    BlurGalleryPin& pin = (*pins_)[selected_];
    if (drag_ == Drag::Move) {
        pin.pos = toFraction(widget);
    } else if (drag_ == Drag::Blur) {
        const QRectF area = imageRect();
        const double docPerWidget = area.width() > 0.0 && documentSize_.width() > 0
                                        ? double(documentSize_.width()) / area.width()
                                        : 1.0;
        pin.blur = qBound(0.0, dragStartBlur_ + (widget.x() - dragStart_.x()) * docPerWidget,
                          kMaxBlur);
    } else if (drag_ != Drag::None) {
        dragIris(widget, pin);
    } else {
        return;
    }
    update();
    emit pinsEdited();
}

void BlurGalleryCanvas::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), kPasteboard);
    const QRectF area = imageRect();
    if (!image_.isNull()) {
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawImage(area, image_);
    }
    if (!uiVisible_ || !pins_ || pins_->isEmpty()) {
        return;
    }
    painter.setRenderHint(QPainter::Antialiasing, true);
    if (iris_) {
        paintIris(painter, pins_->at(selected_));
    }
    // Unselected pins first, the selected one last, so it reads on top.
    for (int pass = 0; pass < 2; ++pass) {
        for (int i = 0; i < pins_->size(); ++i) {
            const bool selected = i == selected_;
            if ((pass == 0) == selected) {
                continue;
            }
            const QPointF at = toWidget(pins_->at(i).pos);
            // The selected pin is the larger circle with the blur handle ring;
            // the others are small target dots, all equally draggable. The
            // ring's white arc shows the pin's blur amount (full at 100 px).
            const double ring = selected ? kRing : kRingSmall;
            const double dot = selected ? kPin : kDot;
            const double ringPen = selected ? 2.8 : 2.0;
            const QColor base = selected ? QColor(70, 70, 70) : QColor(55, 55, 55);
            const QColor fill = selected ? QColor(255, 255, 255) : kMarker;
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(QColor(0, 0, 0, 140), ringPen + 1.6));
            painter.drawEllipse(at, ring, ring);
            painter.setPen(QPen(base, ringPen));
            painter.drawEllipse(at, ring, ring);
            const double span =
                360.0 * std::clamp(pins_->at(i).blur / 100.0, 0.0, 1.0);
            if (span > 0.0) {
                const QRectF circle(at.x() - ring, at.y() - ring, 2.0 * ring, 2.0 * ring);
                painter.setPen(QPen(fill, ringPen, Qt::SolidLine, Qt::FlatCap));
                painter.drawArc(circle, 90 * 16, int(-span * 16.0));
            }
            painter.setPen(QPen(QColor(255, 255, 255, 200), 1.0));
            painter.setBrush(QColor(0x20, 0x20, 0x20, 235));
            painter.drawEllipse(at, dot, dot);
        }
    }
    const BlurGalleryPin& pin = pins_->at(selected_);
    const QPointF at = toWidget(pin.pos);
    const QString text = QStringLiteral("Blur: %1").arg(qRound(pin.blur));
    const QRectF textRect = QFontMetricsF(painter.font()).boundingRect(text);
    QRectF box = textRect.adjusted(-7.0, -3.0, 7.0, 3.0);
    box.moveBottomLeft(QPointF(at.x() - box.width() / 2.0, at.y() - kRing - 5.0));
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0x30, 0x30, 0x30, 225));
    painter.drawRoundedRect(box, 2.0, 2.0);
    painter.setPen(QColor(0xf0, 0xf0, 0xf0));
    painter.drawText(box, Qt::AlignCenter, text);
}

void BlurGalleryCanvas::paintIris(QPainter& painter, const BlurGalleryPin& pin) const
{
    const IrisFrame frame = irisFrame(pin, toWidget(pin.pos), imageRect().width());
    QPainterPath outline;
    const int steps = 144;
    const double power = 2.0 / frame.exponent;
    for (int i = 0; i <= steps; ++i) {
        const double t = 2.0 * M_PI * i / steps;
        const double c = std::cos(t);
        const double s = std::sin(t);
        const QPointF point = frame.at(std::copysign(std::pow(std::abs(c), power), c),
                                       std::copysign(std::pow(std::abs(s), power), s));
        if (i == 0) {
            outline.moveTo(point);
        } else {
            outline.lineTo(point);
        }
    }
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(0, 0, 0, 110), 2.6));
    painter.drawPath(outline);
    painter.setPen(QPen(QColor(220, 220, 220, 230), 1.2));
    painter.drawPath(outline);

    painter.setPen(QPen(QColor(0, 0, 0, 150), 1.0));
    painter.setBrush(QColor(0xf4, 0xf4, 0xf4));
    for (const double sign : {1.0, -1.0}) {
        painter.drawEllipse(frame.at(sign, 0.0), kHandle, kHandle);
        painter.drawEllipse(frame.at(0.0, sign), kHandle, kHandle);
        painter.drawEllipse(frame.at(sign * pin.feather, 0.0), kFeatherDot, kFeatherDot);
        painter.drawEllipse(frame.at(0.0, sign * pin.feather), kFeatherDot, kFeatherDot);
    }
    const QPointF tip = frame.diamond();
    const double d = kHandle + 1.5;
    painter.drawPolygon(QPolygonF({tip + QPointF(0, -d), tip + QPointF(d, 0),
                                   tip + QPointF(0, d), tip + QPointF(-d, 0)}));
}

void BlurGalleryCanvas::mousePressEvent(QMouseEvent* event)
{
    if (!pins_ || !uiVisible_ || event->button() != Qt::LeftButton) {
        QWidget::mousePressEvent(event);
        return;
    }
    const QPointF pos = event->position();
    bool onRing = false;
    const int index = hitPin(pos, &onRing);
    if (index < 0 && grabIrisHandle(pos)) {
        dragStart_ = pos;
        return;
    }
    if (index >= 0) {
        if (index != selected_) {
            selected_ = index;
            emit pinSelected(index);
        }
        drag_ = onRing ? Drag::Blur : Drag::Move;
        dragStart_ = pos;
        dragStartBlur_ = pins_->at(index).blur;
        update();
        return;
    }
    if (imageRect().contains(pos)) {
        // A new pin copies the selected one's amount and Iris outline.
        BlurGalleryPin pin;
        if (selected_ >= 0 && selected_ < pins_->size()) {
            pin = pins_->at(selected_);
        }
        pin.pos = toFraction(pos);
        pins_->append(pin);
        selected_ = pins_->size() - 1;
        drag_ = Drag::Move;
        dragStart_ = pos;
        emit pinAdded(selected_);
        emit pinsEdited();
        update();
        return;
    }
    QWidget::mousePressEvent(event);
}

void BlurGalleryCanvas::mouseMoveEvent(QMouseEvent* event)
{
    if (drag_ == Drag::None) {
        QWidget::mouseMoveEvent(event);
        return;
    }
    dragTo(event->position());
}

void BlurGalleryCanvas::mouseReleaseEvent(QMouseEvent* event)
{
    if (drag_ == Drag::None) {
        QWidget::mouseReleaseEvent(event);
        return;
    }
    dragTo(event->position());
    drag_ = Drag::None;
}

void BlurGalleryCanvas::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) {
        emit deleteRequested();
        return;
    }
    QWidget::keyPressEvent(event);
}

} // namespace pictura
