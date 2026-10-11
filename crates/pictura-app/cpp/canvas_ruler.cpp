#include "canvas_ruler.h"

#include "image_view.h"
#include "theme.h"

#include <QtGui/QActionGroup>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QMenu>

#include <cmath>

namespace pictura {

namespace {

constexpr double kMinMajorScreenPx = 50.0;
constexpr double kMinMinorScreenPx = 4.0;

} // namespace

QString rulerUnitName(RulerUnit unit)
{
    switch (unit) {
    case RulerUnit::Pixels:
        return QObject::tr("Pixels");
    case RulerUnit::Inches:
        return QObject::tr("Inches");
    case RulerUnit::Centimeters:
        return QObject::tr("Centimeters");
    case RulerUnit::Millimeters:
        return QObject::tr("Millimeters");
    case RulerUnit::Points:
        return QObject::tr("Points");
    case RulerUnit::Picas:
        return QObject::tr("Picas");
    case RulerUnit::Percent:
        return QObject::tr("Percent");
    }
    return QString();
}

CanvasRuler::Scale CanvasRuler::scaleFor(RulerUnit unit, double screenPxPerUnit)
{
    if (!(screenPxPerUnit > 0.0)) {
        return {};
    }
    // Fractional steps only for units larger than a pixel, so a pixel ruler
    // never labels a fraction of a pixel.
    double decade = unit == RulerUnit::Pixels ? 1.0 : 0.001;
    Scale scale;
    for (bool found = false; !found; decade *= 10.0) {
        for (const int mantissa : {1, 2, 5}) {
            const double step = mantissa * decade;
            if (step * screenPxPerUnit >= kMinMajorScreenPx) {
                scale.major = step;
                if (unit == RulerUnit::Inches && step == 1.0) {
                    scale.subdivisions = 8;
                } else {
                    scale.subdivisions = mantissa == 1 ? 10 : mantissa == 2 ? 4 : 5;
                }
                found = true;
                break;
            }
        }
    }
    while (scale.subdivisions > 1
           && scale.major / scale.subdivisions * screenPxPerUnit < kMinMinorScreenPx) {
        scale.subdivisions = scale.subdivisions % 2 == 0 ? scale.subdivisions / 2 : 1;
    }
    if (unit == RulerUnit::Pixels) {
        // Never a tick between two pixels.
        while (scale.subdivisions > 1
               && std::fmod(scale.major, scale.subdivisions) != 0.0) {
            scale.subdivisions = scale.subdivisions % 2 == 0 ? scale.subdivisions / 2 : 1;
        }
    }
    return scale;
}

CanvasRuler::CanvasRuler(Qt::Orientation orientation, QWidget* parent)
    : QWidget(parent)
    , orientation_(orientation)
{
    setObjectName(orientation == Qt::Horizontal ? QStringLiteral("canvasRulerHorizontal")
                                                : QStringLiteral("canvasRulerVertical"));
    if (orientation == Qt::Horizontal) {
        setFixedHeight(kThickness);
    } else {
        setFixedWidth(kThickness);
    }
    setCursor(orientation == Qt::Horizontal ? Qt::SplitVCursor : Qt::SplitHCursor);
}

void CanvasRuler::setView(ImageView* view)
{
    view_ = view;
    hasCursor_ = false;
    update();
}

void CanvasRuler::setUnit(RulerUnit unit)
{
    if (unit_ != unit) {
        unit_ = unit;
        update();
    }
}

void CanvasRuler::setTraditionalPoints(bool on)
{
    if (traditionalPoints_ != on) {
        traditionalPoints_ = on;
        update();
    }
}

void CanvasRuler::mouseDoubleClickEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        dragging_ = false;
        emit preferencesRequested();
    }
}

void CanvasRuler::setPpiProvider(std::function<double()> ppi)
{
    ppi_ = std::move(ppi);
    update();
}

double CanvasRuler::pixelsPerUnit() const
{
    const double ppi = ppi_ ? ppi_() : 72.0;
    const double pointsPerInch = traditionalPoints_ ? 72.27 : 72.0;
    switch (unit_) {
    case RulerUnit::Pixels:
        return 1.0;
    case RulerUnit::Inches:
        return ppi;
    case RulerUnit::Centimeters:
        return ppi / 2.54;
    case RulerUnit::Millimeters:
        return ppi / 25.4;
    case RulerUnit::Points:
        return ppi / pointsPerInch;
    case RulerUnit::Picas:
        return ppi / pointsPerInch * 12.0;
    case RulerUnit::Percent:
        if (view_) {
            const QSize size = view_->documentSize();
            return (orientation_ == Qt::Horizontal ? size.width() : size.height()) / 100.0;
        }
        return 1.0;
    }
    return 1.0;
}

void CanvasRuler::contextMenuEvent(QContextMenuEvent* event)
{
    QMenu menu(this);
    auto* group = new QActionGroup(&menu);
    for (int i = 0; i < kRulerUnitCount; ++i) {
        const auto unit = RulerUnit(i);
        QAction* action = menu.addAction(rulerUnitName(unit));
        action->setCheckable(true);
        action->setChecked(unit == unit_);
        group->addAction(action);
        connect(action, &QAction::triggered, this, [this, unit]() { emit unitChosen(unit); });
    }
    menu.exec(event->globalPos());
}

void CanvasRuler::setCursorPosition(const QPointF& imagePos)
{
    cursor_ = imagePos;
    hasCursor_ = true;
    update();
}

double CanvasRuler::toRuler(double position) const
{
    const QPoint shift = view_->mapToGlobal(QPoint(0, 0)) - mapToGlobal(QPoint(0, 0));
    if (orientation_ == Qt::Horizontal) {
        return position * view_->zoom() + view_->offset().x() + shift.x();
    }
    return position * view_->zoom() + view_->offset().y() + shift.y();
}

void CanvasRuler::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), Theme::panelColor());
    const QColor ink = palette().color(QPalette::WindowText);
    const bool horizontal = orientation_ == Qt::Horizontal;
    const int length = horizontal ? width() : height();
    painter.setPen(ink);
    if (horizontal) {
        painter.drawLine(0, height() - 1, width(), height() - 1);
    } else {
        painter.drawLine(width() - 1, 0, width() - 1, height());
    }
    if (!view_ || !view_->hasDocument()) {
        return;
    }

    const double pxPerUnit = pixelsPerUnit();
    const double screenPerUnit = pxPerUnit * view_->zoom();
    const Scale scale = scaleFor(unit_, screenPerUnit);
    const double minor = scale.major / scale.subdivisions;
    const double origin = toRuler(0.0);
    const auto first = qint64(std::floor(-origin / screenPerUnit / minor)) - 1;
    const auto last = qint64(std::ceil((length - origin) / screenPerUnit / minor)) + 1;
    QFont font = painter.font();
    font.setPixelSize(9);
    painter.setFont(font);
    const int lineHeight = painter.fontMetrics().height() - 2;
    for (qint64 k = first; k <= last; ++k) {
        const double at = std::floor(toRuler(k * minor * pxPerUnit)) + 0.5;
        const int sub = scale.subdivisions;
        const bool major = k % sub == 0;
        const bool half = !major && sub % 2 == 0 && k % (sub / 2) == 0;
        const bool quarter = !major && !half && sub % 4 == 0 && k % (sub / 4) == 0;
        const double tick = major     ? kThickness
            : half                    ? kThickness / 2.0
            : quarter                 ? kThickness * 3.0 / 8.0
                                      : kThickness / 4.0;
        if (horizontal) {
            painter.drawLine(QPointF(at, kThickness - tick), QPointF(at, kThickness));
        } else {
            painter.drawLine(QPointF(kThickness - tick, at), QPointF(kThickness, at));
        }
        if (!major) {
            continue;
        }
        // Distance from the origin, unsigned, as CS6 labels both sides of 0.
        const QString label = QString::number(std::abs(double(k / sub) * scale.major), 'g', 12);
        if (horizontal) {
            painter.drawText(QPointF(at + 2, lineHeight), label);
        } else {
            // Digits stacked down the ruler, as CS6 draws them.
            for (int i = 0; i < label.size(); ++i) {
                painter.drawText(QPointF(2, at + 1 + (i + 1) * lineHeight), label.at(i));
            }
        }
    }

    if (hasCursor_) {
        const double at =
            std::floor(toRuler(horizontal ? cursor_.x() : cursor_.y())) + 0.5;
        painter.setPen(QPen(ink, 0, Qt::DotLine));
        if (horizontal) {
            painter.drawLine(QPointF(at, 0), QPointF(at, kThickness));
        } else {
            painter.drawLine(QPointF(0, at), QPointF(kThickness, at));
        }
    }
}

std::optional<double> CanvasRuler::guidePositionAt(const QPointF& globalPos) const
{
    if (!view_ || !view_->hasDocument()) {
        return std::nullopt;
    }
    const QPointF local = view_->mapFromGlobal(globalPos);
    if (!QRectF(view_->rect()).contains(local)) {
        return std::nullopt;
    }
    const QPointF image = view_->widgetToImage(local);
    return std::round(orientation_ == Qt::Horizontal ? image.y() : image.x());
}

void CanvasRuler::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton || !view_ || !view_->hasDocument()) {
        QWidget::mousePressEvent(event);
        return;
    }
    dragging_ = true;
    event->accept();
}

void CanvasRuler::mouseMoveEvent(QMouseEvent* event)
{
    if (!dragging_) {
        QWidget::mouseMoveEvent(event);
        return;
    }
    if (const auto position = guidePositionAt(event->globalPosition())) {
        view_->setGuidePreview({orientation_ == Qt::Vertical, *position});
    } else if (view_) {
        view_->clearGuidePreview();
    }
}

void CanvasRuler::mouseReleaseEvent(QMouseEvent* event)
{
    if (!dragging_ || event->button() != Qt::LeftButton) {
        QWidget::mouseReleaseEvent(event);
        return;
    }
    dragging_ = false;
    if (view_) {
        view_->clearGuidePreview();
    }
    if (const auto position = guidePositionAt(event->globalPosition())) {
        emit guideDropped(orientation_ == Qt::Vertical, *position);
    }
}

} // namespace pictura
