#include "blur_gallery_canvas.h"

#include "icons.h"

#include <QtCore/QLineF>
#include <QtCore/QtMath>
#include <QtGui/QFontMetrics>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygonF>
#include <QtGui/QResizeEvent>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QScrollBar>

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
const double kMaxTiltExtent = 10.0;
const double kMaxSpinAngle = 360.0;
// The preview zoom bounds and one wheel notch's factor, as Liquify's.
const double kMinZoom = 0.01;
const double kMaxZoom = 32.0;
const double kWheelZoom = 1.25;
// The preview scrollbars' thickness, inside the 24 px pasteboard margin.
const int kScrollBar = 14;

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
    // With the side column, this keeps the workspace at least ~760 px wide.
    setMinimumSize(480, 320);
    setFocusPolicy(Qt::ClickFocus);
    setMouseTracking(true);
    hbar_ = new QScrollBar(Qt::Horizontal, this);
    hbar_->setObjectName(QStringLiteral("blurGalleryHScroll"));
    vbar_ = new QScrollBar(Qt::Vertical, this);
    vbar_->setObjectName(QStringLiteral("blurGalleryVScroll"));
    for (QScrollBar* bar : {hbar_, vbar_}) {
        connect(bar, &QScrollBar::valueChanged, this, [this] { update(); });
        bar->hide();
    }
}

void BlurGalleryCanvas::setImage(const QImage& image)
{
    image_ = image;
    updateScrollBars();
    update();
}

void BlurGalleryCanvas::setDocumentSize(const QSize& size)
{
    documentSize_ = size;
    updateScrollBars();
    if (zoom_ <= 0.0) {
        // The fit zoom follows the new document size.
        emit zoomChanged();
    }
    update();
}

double BlurGalleryCanvas::zoom() const
{
    return zoom_ > 0.0 ? zoom_ : fitZoom();
}

QRectF BlurGalleryCanvas::roomRect() const
{
    return QRectF(rect()).adjusted(kMargin, kMargin, -kMargin, -kMargin);
}

double BlurGalleryCanvas::fitZoom() const
{
    const QRectF room = roomRect();
    const QSizeF doc = documentPixels();
    if (room.isEmpty() || doc.isEmpty()) {
        return 1.0;
    }
    return std::clamp(qMin(room.width() / doc.width(), room.height() / doc.height()), kMinZoom,
                      kMaxZoom);
}

void BlurGalleryCanvas::setZoom(double zoom)
{
    zoomAbout(zoom, roomRect().center());
}

void BlurGalleryCanvas::zoomBy(double factor)
{
    setZoom(zoom() * factor);
}

void BlurGalleryCanvas::zoomAbout(double zoom, const QPointF& anchor)
{
    const QRectF before = imageRect();
    const double next = std::clamp(zoom, kMinZoom, kMaxZoom);
    if (before.width() <= 0.0 || before.height() <= 0.0) {
        zoom_ = next;
        updateScrollBars();
        update();
        emit zoomChanged();
        return;
    }
    // The document fraction under the anchor stays under it after the zoom.
    const QPointF at((anchor.x() - before.x()) / before.width(),
                     (anchor.y() - before.y()) / before.height());
    zoom_ = next;
    updateScrollBars();
    const QSizeF doc = documentPixels();
    const QSizeF shown(doc.width() * next, doc.height() * next);
    const QRectF room = roomRect();
    // A picture that overflows scrolls, so solve the scroll value that puts
    // `at` back under `anchor`; a fitted axis stays centred.
    if (shown.width() > room.width() && hbar_) {
        hbar_->setValue(qRound(room.left() - anchor.x() + at.x() * shown.width()));
    }
    if (shown.height() > room.height() && vbar_) {
        vbar_->setValue(qRound(room.top() - anchor.y() + at.y() * shown.height()));
    }
    update();
    emit zoomChanged();
}

void BlurGalleryCanvas::fitInView()
{
    zoom_ = 0.0;
    if (hbar_) {
        hbar_->setValue(0);
    }
    if (vbar_) {
        vbar_->setValue(0);
    }
    updateScrollBars();
    update();
    emit zoomChanged();
}

QSizeF BlurGalleryCanvas::documentPixels() const
{
    return documentSize_.isEmpty() ? QSizeF(image_.size()) : QSizeF(documentSize_);
}

void BlurGalleryCanvas::updateScrollBars()
{
    if (!hbar_ || !vbar_) {
        return;
    }
    const QRectF room = roomRect();
    const QSizeF doc = documentPixels();
    const QSizeF shown(doc.width() * zoom(), doc.height() * zoom());
    const int maxH = qMax(0, static_cast<int>(std::ceil(shown.width() - room.width())));
    const int maxV = qMax(0, static_cast<int>(std::ceil(shown.height() - room.height())));
    hbar_->setRange(0, maxH);
    hbar_->setPageStep(qMax(1, qRound(room.width())));
    hbar_->setSingleStep(qMax(1, qRound(room.width() / 10.0)));
    vbar_->setRange(0, maxV);
    vbar_->setPageStep(qMax(1, qRound(room.height())));
    vbar_->setSingleStep(qMax(1, qRound(room.height() / 10.0)));
    hbar_->setVisible(maxH > 0);
    vbar_->setVisible(maxV > 0);
    hbar_->setGeometry(0, qMax(0, height() - kScrollBar),
                       qMax(0, width() - (vbar_->isVisible() ? kScrollBar : 0)), kScrollBar);
    vbar_->setGeometry(qMax(0, width() - kScrollBar), 0, kScrollBar,
                       qMax(0, height() - (hbar_->isVisible() ? kScrollBar : 0)));
}

void BlurGalleryCanvas::setMode(BlurGalleryMode mode)
{
    finishPath();
    mode_ = mode;
    update();
}

void BlurGalleryCanvas::setPins(QList<BlurGalleryPin>* pins, int selected)
{
    finishPath();
    pins_ = pins;
    selected_ = pins_ && selected >= 0 && selected < pins_->size() ? selected : 0;
    selectedPoint_ = -1;
    drag_ = Drag::None;
    unsetCursor();
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
    if (index != selected_) {
        selectedPoint_ = -1;
    }
    selected_ = index;
    update();
}

void BlurGalleryCanvas::setSelectedPoint(int point)
{
    const bool valid = pins_ && selected_ >= 0 && selected_ < pins_->size() && point >= 0
                       && point < pins_->at(selected_).points.size();
    selectedPoint_ = valid ? point : -1;
    update();
}

QSize BlurGalleryCanvas::sizeHint() const
{
    return {640, 480};
}

QRectF BlurGalleryCanvas::imageRect() const
{
    const QRectF room = roomRect();
    if (room.isEmpty()) {
        return room;
    }
    // The document's aspect and zoom, so the draft and full previews place
    // identically. A fitted axis centres; an overflowing one follows its
    // scrollbar.
    const QSizeF doc = documentPixels();
    if (doc.isEmpty()) {
        return room;
    }
    const QSizeF shown(doc.width() * zoom(), doc.height() * zoom());
    const double x = (shown.width() <= room.width() || !hbar_)
                         ? room.center().x() - shown.width() / 2.0
                         : room.left() - hbar_->value();
    const double y = (shown.height() <= room.height() || !vbar_)
                         ? room.center().y() - shown.height() / 2.0
                         : room.top() - vbar_->value();
    return QRectF(x, y, shown.width(), shown.height());
}

QPointF BlurGalleryCanvas::toWidget(const QPointF& fraction) const
{
    const QRectF area = imageRect();
    return {area.x() + fraction.x() * area.width(), area.y() + fraction.y() * area.height()};
}

QPointF BlurGalleryCanvas::markerAt(const BlurGalleryPin& pin) const
{
    return toWidget(mode_ == BlurGalleryMode::Spin ? pin.pos + pin.pivot : pin.pos);
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

// A spin's outline is always an ellipse (`round`), whatever its squareness.
IrisFrame irisFrame(const BlurGalleryPin& pin, const QPointF& centre, double widthPx, bool round)
{
    const double radians = qDegreesToRadians(pin.angle);
    IrisFrame frame;
    frame.centre = centre;
    frame.along = {std::cos(radians), std::sin(radians)};
    frame.across = {-std::sin(radians), std::cos(radians)};
    frame.a = pin.radii.width() * widthPx;
    frame.b = pin.radii.height() * widthPx;
    frame.exponent = round ? 2.0 : irisExponent(pin.squareness);
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
        if (QLineF(widget, markerAt(pins_->at(i))).length() <= kRingSmall + 2.0) {
            return i;
        }
    }
    if (selected_ >= 0 && selected_ < pins_->size()
        && std::abs(QLineF(widget, markerAt(pins_->at(selected_))).length() - kRing) <= kGrab) {
        *onRing = true;
        return selected_;
    }
    return -1;
}

bool BlurGalleryCanvas::grabIrisHandle(const QPointF& widget)
{
    const bool spin = mode_ == BlurGalleryMode::Spin;
    if ((mode_ != BlurGalleryMode::Iris && !spin) || !pins_ || selected_ < 0
        || selected_ >= pins_->size()) {
        return false;
    }
    const BlurGalleryPin& pin = pins_->at(selected_);
    const IrisFrame frame = irisFrame(pin, toWidget(pin.pos), imageRect().width(), spin);
    const auto near = [&](const QPointF& handle, double size) {
        return QLineF(widget, handle).length() <= size + kGrab;
    };
    if (!spin && near(frame.diamond(), kHandle)) {
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
    const IrisFrame frame =
        irisFrame(pin, toWidget(pin.pos), widthPx, mode_ == BlurGalleryMode::Spin);
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
    if (drag_ == Drag::Point || drag_ == Drag::Path) {
        dragPath(widget);
        update();
        emit pinsEdited();
        return;
    }
    BlurGalleryPin& pin = (*pins_)[selected_];
    const bool spin = mode_ == BlurGalleryMode::Spin;
    if (drag_ == Drag::Move) {
        // The marker follows the pointer; a spin's outline keeps its offset.
        pin.pos = toFraction(widget) - (spin ? pin.pivot : QPointF());
    } else if (drag_ == Drag::Pivot) {
        pin.pivot = toFraction(widget) - pin.pos;
    } else if (drag_ == Drag::Blur && spin) {
        // A degree of Blur Angle per pixel dragged.
        pin.blur = qBound(0.0, dragStartBlur_ + widget.x() - dragStart_.x(), kMaxSpinAngle);
    } else if (drag_ == Drag::Blur) {
        const QRectF area = imageRect();
        const double docPerWidget = area.width() > 0.0 && documentSize_.width() > 0
                                        ? double(documentSize_.width()) / area.width()
                                        : 1.0;
        pin.blur = qBound(0.0, dragStartBlur_ + (widget.x() - dragStart_.x()) * docPerWidget,
                          kMaxBlur);
    } else if (drag_ == Drag::Focus || drag_ == Drag::Fade || drag_ == Drag::Rotate) {
        dragTilt(widget, pin);
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
    if (mode_ == BlurGalleryMode::Path) {
        paintPaths(painter);
        return;
    }
    const bool spin = mode_ == BlurGalleryMode::Spin;
    if (mode_ == BlurGalleryMode::Iris || spin) {
        paintIris(painter, pins_->at(selected_));
    } else if (mode_ == BlurGalleryMode::TiltShift) {
        paintTilt(painter, pins_->at(selected_));
    }
    // Unselected pins first, the selected one last, so it reads on top.
    for (int pass = 0; pass < 2; ++pass) {
        for (int i = 0; i < pins_->size(); ++i) {
            const bool selected = i == selected_;
            if ((pass == 0) == selected) {
                continue;
            }
            const QPointF at = markerAt(pins_->at(i));
            // The selected pin is the larger circle with the blur handle ring;
            // the others are small target dots, all equally draggable. The
            // ring's white arc shows the pin's blur amount (full at 100 px),
            // or a spin's Blur Angle (full at 360°).
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
                360.0 * std::clamp(pins_->at(i).blur / (spin ? kMaxSpinAngle : 100.0), 0.0, 1.0);
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
    const QPointF at = markerAt(pin);
    const QString text = spin ? QStringLiteral("Blur Angle: %1°").arg(qRound(pin.blur))
                              : QStringLiteral("Blur: %1").arg(qRound(pin.blur));
    const QRectF textRect = QFontMetricsF(painter.font()).boundingRect(text);
    QRectF box = textRect.adjusted(-7.0, -3.0, 7.0, 3.0);
    box.moveBottomLeft(QPointF(at.x() - box.width() / 2.0, at.y() - kRing - 5.0));
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0x30, 0x30, 0x30, 225));
    painter.drawRoundedRect(box, 2.0, 2.0);
    painter.setPen(QColor(0xf0, 0xf0, 0xf0));
    painter.drawText(box, Qt::AlignCenter, text);
    // The rotation dots sit on top so the amount label never hides one.
    if (mode_ == BlurGalleryMode::TiltShift) {
        paintTiltDots(painter, pin);
    }
}

void BlurGalleryCanvas::paintIris(QPainter& painter, const BlurGalleryPin& pin) const
{
    const bool spin = mode_ == BlurGalleryMode::Spin;
    const IrisFrame frame = irisFrame(pin, toWidget(pin.pos), imageRect().width(), spin);
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
    if (spin) {
        return;
    }
    const QPointF tip = frame.diamond();
    const double d = kHandle + 1.5;
    painter.drawPolygon(QPolygonF({tip + QPointF(0, -d), tip + QPointF(d, 0),
                                   tip + QPointF(0, d), tip + QPointF(-d, 0)}));
}

namespace {

// A pin's band in widget pixels: its centre, the unit axes along and across
// the band (across points below it), and the picture height the extents
// scale by.
struct TiltFrame {
    QPointF centre;
    QPointF along;
    QPointF across;
    double heightPx = 0.0;

    // The distance of the line on `side` (-1 above, +1 below) from the
    // centre line, `outer` for the dashed fade line.
    double offset(const BlurGalleryPin& pin, int side, bool outer) const
    {
        const int k = side < 0 ? 0 : 1;
        return (pin.focus[k] + (outer ? pin.fade[k] : 0.0)) * heightPx;
    }
    QPointF dot(const BlurGalleryPin& pin, int side) const
    {
        return centre + across * (side * offset(pin, side, false));
    }
};

TiltFrame tiltFrame(const BlurGalleryPin& pin, const QPointF& centre, double heightPx)
{
    const double radians = qDegreesToRadians(pin.angle);
    TiltFrame frame;
    frame.centre = centre;
    frame.along = {std::cos(radians), std::sin(radians)};
    frame.across = {-std::sin(radians), std::cos(radians)};
    frame.heightPx = heightPx;
    return frame;
}

} // namespace

BlurGalleryCanvas::Drag BlurGalleryCanvas::hitTilt(const QPointF& widget, double* sign) const
{
    if (mode_ != BlurGalleryMode::TiltShift || !pins_ || selected_ < 0
        || selected_ >= pins_->size()) {
        return Drag::None;
    }
    const BlurGalleryPin& pin = pins_->at(selected_);
    const TiltFrame frame = tiltFrame(pin, toWidget(pin.pos), imageRect().height());
    const double v = dot(widget - frame.centre, frame.across);
    const int side = v < 0.0 ? -1 : 1;
    for (const int s : {-1, 1}) {
        if (QLineF(widget, frame.dot(pin, s)).length() <= kHandle + kGrab) {
            *sign = s;
            return Drag::Rotate;
        }
    }
    // Lines span the picture; a press past a coincident solid line grabs the
    // dashed one so a zero fade can be pulled open.
    const double solid = frame.offset(pin, side, false);
    const double dashed = frame.offset(pin, side, true);
    const bool outside = std::abs(v) >= solid;
    const bool nearSolid = std::abs(std::abs(v) - solid) <= kGrab;
    const bool nearDashed = std::abs(std::abs(v) - dashed) <= kGrab;
    if (!imageRect().adjusted(-kGrab, -kGrab, kGrab, kGrab).contains(widget)
        || (!nearSolid && !nearDashed)) {
        return Drag::None;
    }
    *sign = side;
    return nearDashed && (outside || !nearSolid) ? Drag::Fade : Drag::Focus;
}

bool BlurGalleryCanvas::grabTiltHandle(const QPointF& widget)
{
    const Drag hit = hitTilt(widget, &dragSign_);
    if (hit == Drag::None) {
        return false;
    }
    drag_ = hit;
    return true;
}

void BlurGalleryCanvas::updateTiltCursor(Drag hit, const QPointF& widget)
{
    if (hit == Drag::None) {
        unsetCursor();
        return;
    }
    const BlurGalleryPin& pin = pins_->at(selected_);
    if (hit == Drag::Rotate) {
        setCursor(rotateCursor(toWidget(pin.pos), widget));
        return;
    }
    // A line moves across the band: the double arrow follows its normal,
    // snapped to the four resize cursors (y-down: 45° points down-right).
    const double normal = std::fmod(std::fmod(pin.angle + 90.0, 180.0) + 180.0, 180.0);
    const Qt::CursorShape shapes[] = {Qt::SizeHorCursor, Qt::SizeFDiagCursor, Qt::SizeVerCursor,
                                      Qt::SizeBDiagCursor};
    setCursor(shapes[int(std::lround(normal / 45.0)) % 4]);
}

void BlurGalleryCanvas::dragTilt(const QPointF& widget, BlurGalleryPin& pin) const
{
    const double heightPx = imageRect().height();
    if (heightPx <= 0.0) {
        return;
    }
    const TiltFrame frame = tiltFrame(pin, toWidget(pin.pos), heightPx);
    const QPointF offset = widget - frame.centre;
    const int k = dragSign_ < 0.0 ? 0 : 1;
    if (drag_ == Drag::Rotate) {
        // The dot follows the pointer round the pin; the band turns with it.
        const double length = std::hypot(offset.x(), offset.y());
        if (length <= 0.0) {
            return;
        }
        const QPointF axis = offset * (dragSign_ / length);
        pin.angle = qRadiansToDegrees(std::atan2(-axis.x(), axis.y()));
        return;
    }
    const double reach = std::clamp(dragSign_ * dot(offset, frame.across) / heightPx, 0.0,
                                    kMaxTiltExtent);
    if (drag_ == Drag::Focus) {
        // The dashed line stays put until the solid line pushes it.
        const double outer = pin.focus[k] + pin.fade[k];
        pin.focus[k] = reach;
        pin.fade[k] = std::max(0.0, outer - reach);
    } else {
        pin.fade[k] = std::clamp(reach - pin.focus[k], 0.0, kMaxTiltExtent);
    }
}

void BlurGalleryCanvas::paintTilt(QPainter& painter, const BlurGalleryPin& pin) const
{
    const QRectF area = imageRect();
    const TiltFrame frame = tiltFrame(pin, toWidget(pin.pos), area.height());
    // Long enough to cross the picture at any angle from any pin position.
    const double span = 4.0 * std::hypot(area.width(), area.height());
    painter.save();
    painter.setClipRect(area);
    painter.setBrush(Qt::NoBrush);
    for (const int side : {-1, 1}) {
        for (const bool outer : {false, true}) {
            const QPointF mid = frame.centre + frame.across * (side * frame.offset(pin, side, outer));
            const QLineF line(mid - frame.along * span, mid + frame.along * span);
            painter.setPen(QPen(QColor(0, 0, 0, 110), 2.6));
            painter.drawLine(line);
            QPen pen(QColor(220, 220, 220, 230), 1.4);
            if (outer) {
                pen.setDashPattern({6.0, 4.0});
            }
            painter.setPen(pen);
            painter.drawLine(line);
        }
    }
    painter.restore();
}

void BlurGalleryCanvas::paintTiltDots(QPainter& painter, const BlurGalleryPin& pin) const
{
    const TiltFrame frame = tiltFrame(pin, toWidget(pin.pos), imageRect().height());
    painter.setPen(QPen(QColor(0, 0, 0, 150), 1.0));
    painter.setBrush(QColor(0xf4, 0xf4, 0xf4));
    for (const int side : {-1, 1}) {
        painter.drawEllipse(frame.dot(pin, side), kHandle, kHandle);
    }
}

void BlurGalleryCanvas::mousePressEvent(QMouseEvent* event)
{
    // The scroll wheel pressed and dragged pans the zoomed preview.
    if (event->button() == Qt::MiddleButton) {
        if (hbar_->maximum() > 0 || vbar_->maximum() > 0) {
            panning_ = true;
            panStart_ = event->position();
            panValues_ = QPoint(hbar_->value(), vbar_->value());
            setCursor(Qt::ClosedHandCursor);
        }
        event->accept();
        return;
    }
    if (!pins_ || !uiVisible_ || event->button() != Qt::LeftButton) {
        QWidget::mousePressEvent(event);
        return;
    }
    const QPointF pos = event->position();
    if (mode_ == BlurGalleryMode::Path) {
        pressPath(pos, event->modifiers());
        return;
    }
    bool onRing = false;
    const int index = hitPin(pos, &onRing);
    if (index < 0 && (grabIrisHandle(pos) || grabTiltHandle(pos))) {
        dragStart_ = pos;
        return;
    }
    if (index >= 0) {
        if (index != selected_) {
            selected_ = index;
            emit pinSelected(index);
        }
        const bool pivot =
            mode_ == BlurGalleryMode::Spin && (event->modifiers() & Qt::AltModifier);
        drag_ = onRing ? Drag::Blur : (pivot ? Drag::Pivot : Drag::Move);
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
        pin.pivot = QPointF();
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
    if (panning_) {
        const QPointF delta = event->position() - panStart_;
        hbar_->setValue(panValues_.x() - qRound(delta.x()));
        vbar_->setValue(panValues_.y() - qRound(delta.y()));
        event->accept();
        return;
    }
    if (drag_ == Drag::None && mode_ == BlurGalleryMode::Path && pins_ && uiVisible_) {
        // The path being placed follows the pointer; hover shows the pin
        // that places a stop.
        if (building_) {
            moveFloatingStop(event->position());
        }
        updatePathCursor(event->position());
        QWidget::mouseMoveEvent(event);
        return;
    }
    if (drag_ == Drag::None) {
        // Hover: a Tilt-Shift line or dot shows the drag it would start; a
        // pin wins over a line it sits on, as a press does.
        const QPointF pos = event->position();
        bool onRing = false;
        double sign = 1.0;
        const bool live = uiVisible_ && hitPin(pos, &onRing) < 0;
        updateTiltCursor(live ? hitTilt(pos, &sign) : Drag::None, pos);
        QWidget::mouseMoveEvent(event);
        return;
    }
    if (drag_ == Drag::Rotate) {
        setCursor(rotateCursor(toWidget(pins_->at(selected_).pos), event->position()));
    }
    dragTo(event->position());
}

void BlurGalleryCanvas::mouseReleaseEvent(QMouseEvent* event)
{
    if (panning_ && event->button() == Qt::MiddleButton) {
        panning_ = false;
        unsetCursor();
        event->accept();
        return;
    }
    if (drag_ == Drag::None) {
        QWidget::mouseReleaseEvent(event);
        return;
    }
    dragTo(event->position());
    drag_ = Drag::None;
    emit dragFinished();
}

void BlurGalleryCanvas::mouseDoubleClickEvent(QMouseEvent* event)
{
    // The double-click's first press already pinned this stop. A stray
    // double-click never starts a path: Qt delivers its second press as this
    // event, so forwarding it would.
    if (mode_ == BlurGalleryMode::Path) {
        finishPath();
        return;
    }
    QWidget::mouseDoubleClickEvent(event);
}

void BlurGalleryCanvas::wheelEvent(QWheelEvent* event)
{
    const int delta = event->angleDelta().y();
    if (delta == 0) {
        QWidget::wheelEvent(event);
        return;
    }
    // A notch zooms about the pointer, so the detail under it stays put.
    zoomAbout(zoom() * (delta > 0 ? kWheelZoom : 1.0 / kWheelZoom), event->position());
    event->accept();
}

void BlurGalleryCanvas::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    updateScrollBars();
    // A fit view re-fits to the new room; the dialog's zoom field follows.
    if (zoom_ <= 0.0) {
        emit zoomChanged();
    }
}

void BlurGalleryCanvas::keyPressEvent(QKeyEvent* event)
{
    const int key = event->key();
    if (building_
        && (key == Qt::Key_Return || key == Qt::Key_Enter || key == Qt::Key_Escape)) {
        finishPath();
        return;
    }
    finishPath();
    if (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) {
        emit deleteRequested();
        return;
    }
    QWidget::keyPressEvent(event);
}

} // namespace pictura

#include "moc_blur_gallery_canvas.cpp"
