#include "curve_widget.h"

#include "theme.h"

#include <QtCore/QRectF>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QKeyEvent>
#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPaintEvent>

#include <algorithm>
#include <cmath>

namespace pictura {
namespace {

// Monotone cubic spline (Fritsch-Carlson), evaluated at each integer input.
void splineInterpolate(const QVector<QPointF>& pts, float out[256])
{
    const int n = pts.size();
    if (n == 0) {
        for (int i = 0; i < 256; ++i) {
            out[i] = i / 255.0f;
        }
        return;
    }
    if (n == 1) {
        for (int i = 0; i < 256; ++i) {
            out[i] = qBound(0.0f, static_cast<float>(pts[0].y()), 1.0f);
        }
        return;
    }

    QVector<double> dx(n - 1), dy(n - 1), m(n);
    for (int i = 0; i < n - 1; ++i) {
        dx[i] = pts[i + 1].x() - pts[i].x();
        dy[i] = pts[i + 1].y() - pts[i].y();
    }
    QVector<double> slopes(n - 1);
    for (int i = 0; i < n - 1; ++i) {
        slopes[i] = dx[i] > 1e-9 ? dy[i] / dx[i] : 0.0;
    }

    m[0] = slopes[0];
    m[n - 1] = slopes[n - 2];
    for (int i = 1; i < n - 1; ++i) {
        m[i] = (slopes[i - 1] + slopes[i]) * 0.5;
    }

    for (int i = 0; i < n - 1; ++i) {
        if (std::abs(slopes[i]) < 1e-9) {
            m[i] = 0;
            m[i + 1] = 0;
        } else {
            double a = m[i] / slopes[i];
            double b = m[i + 1] / slopes[i];
            double h = std::hypot(a, b);
            if (h > 3.0) {
                double t = 3.0 / h;
                m[i] = t * a * slopes[i];
                m[i + 1] = t * b * slopes[i];
            }
        }
    }

    for (int ix = 0; ix < 256; ++ix) {
        double x = ix / 255.0;
        if (x <= pts[0].x()) {
            out[ix] = qBound(0.0f, static_cast<float>(pts[0].y()), 1.0f);
            continue;
        }
        if (x >= pts[n - 1].x()) {
            out[ix] = qBound(0.0f, static_cast<float>(pts[n - 1].y()), 1.0f);
            continue;
        }
        int seg = 0;
        for (int i = n - 2; i >= 0; --i) {
            if (x >= pts[i].x()) {
                seg = i;
                break;
            }
        }
        double h = dx[seg];
        if (h < 1e-12) {
            out[ix] = qBound(0.0f, static_cast<float>(pts[seg].y()), 1.0f);
            continue;
        }
        double t = (x - pts[seg].x()) / h;
        double t2 = t * t, t3 = t2 * t;
        double h00 = 2 * t3 - 3 * t2 + 1;
        double h10 = t3 - 2 * t2 + t;
        double h01 = -2 * t3 + 3 * t2;
        double h11 = t3 - t2;
        double val = h00 * pts[seg].y() + h10 * h * m[seg] + h01 * pts[seg + 1].y()
                     + h11 * h * m[seg + 1];
        out[ix] = qBound(0.0f, static_cast<float>(val), 1.0f);
    }
}

constexpr qreal kRamp = 10.0;
constexpr qreal kGap = 4.0;

bool byInput(const QPointF& a, const QPointF& b) { return a.x() < b.x(); }

} // namespace

CurveWidget::CurveWidget(QWidget* parent)
    : QWidget(parent)
{
    setMinimumSize(160, 160);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    setFocusPolicy(Qt::ClickFocus);
    resetCurve();
}

QSize CurveWidget::sizeHint() const
{
    const int extra = showRamps_ ? int(kRamp + kGap) : 0;
    return {258 + extra, 258 + extra};
}

void CurveWidget::resetCurve()
{
    points_ = {QPointF(0.0, 0.0), QPointF(1.0, 1.0)};
    selected_ = -1;
    interpolate();
    update();
}

void CurveWidget::setPoints(const QVector<QPointF>& pts)
{
    points_ = pts;
    selected_ = -1;
    interpolate();
    update();
    emit curveChanged();
    emit selectionChanged();
}

QString CurveWidget::pointsText() const
{
    QStringList pairs;
    int lastX = -1;
    for (const QPointF& p : points_) {
        const int x = qBound(0, qRound(p.x() * 255.0), 255);
        const int y = qBound(0, qRound(p.y() * 255.0), 255);
        if (x > lastX) {
            pairs << QStringLiteral("%1,%2").arg(x).arg(y);
            lastX = x;
        }
    }
    return pairs.join(QLatin1Char(' '));
}

void CurveWidget::setPointsText(const QString& text)
{
    QVector<QPointF> points;
    for (const QString& pair : text.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        const QStringList xy = pair.split(QLatin1Char(','));
        if (xy.size() == 2) {
            points.append(QPointF(xy[0].toInt() / 255.0, xy[1].toInt() / 255.0));
        }
    }
    const QSignalBlocker block(this);
    setPoints(points);
}

void CurveWidget::setBins(const std::array<int, 256>& bins)
{
    histo_ = bins;
    update();
}

void CurveWidget::buildLut(uint8_t lut[256]) const
{
    for (int i = 0; i < 256; ++i) {
        lut[i] = static_cast<uint8_t>(qBound(0.0f, curve_[i] * 255.0f + 0.5f, 255.0f));
    }
}

void CurveWidget::interpolate()
{
    std::sort(points_.begin(), points_.end(), byInput);
    splineInterpolate(points_, curve_);
}

void CurveWidget::select(int index)
{
    if (index != selected_) {
        selected_ = index;
        emit selectionChanged();
    }
    update();
}

void CurveWidget::removeAt(int index)
{
    // Endpoints stay: the curve always spans the full input range.
    if (index <= 0 || index >= points_.size() - 1) {
        return;
    }
    points_.removeAt(index);
    selected_ = -1;
    interpolate();
    update();
    emit curveChanged();
    emit selectionChanged();
}

void CurveWidget::moveSelected(int input, int output)
{
    if (selected_ < 0 || selected_ >= points_.size()) {
        return;
    }
    const int last = int(points_.size()) - 1;
    double x = qBound(0, input, 255) / 255.0;
    if (selected_ == 0) {
        x = points_[0].x();
    } else if (selected_ == last) {
        x = points_[last].x();
    } else {
        const double step = 1.0 / 255.0;
        x = std::clamp(x, points_[selected_ - 1].x() + step, points_[selected_ + 1].x() - step);
    }
    points_[selected_] = QPointF(x, qBound(0, output, 255) / 255.0);
    interpolate();
    update();
    emit curveChanged();
}

QRectF CurveWidget::plot() const
{
    // An endpoint's square overhangs the face by 4 px, so inset by that much.
    constexpr qreal inset = 5.0;
    const qreal left = showRamps_ ? kRamp + kGap : inset;
    const qreal bottom = showRamps_ ? kRamp + kGap : inset;
    const qreal side =
        std::max<qreal>(1.0, std::min(width() - left - inset, height() - bottom - inset));
    return {left, inset, side, side};
}

QPointF CurveWidget::toWidget(QPointF p) const
{
    const QRectF r = plot();
    return {r.left() + p.x() * r.width(), r.top() + (1.0 - p.y()) * r.height()};
}

QPointF CurveWidget::fromWidget(QPointF p) const
{
    const QRectF r = plot();
    return {(p.x() - r.left()) / r.width(), 1.0 - (p.y() - r.top()) / r.height()};
}

void CurveWidget::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const QRectF r = plot();
    p.fillRect(r, Theme::tokenColor(QStringLiteral("surface-input")));

    if (showRamps_) {
        QLinearGradient vertical(r.left(), r.bottom(), r.left(), r.top());
        vertical.setColorAt(0.0, Qt::black);
        vertical.setColorAt(1.0, Qt::white);
        p.fillRect(QRectF(0.0, r.top(), kRamp, r.height()), vertical);
        QLinearGradient horizontal(r.left(), 0.0, r.right(), 0.0);
        horizontal.setColorAt(0.0, Qt::black);
        horizontal.setColorAt(1.0, Qt::white);
        p.fillRect(QRectF(r.left(), r.bottom() + kGap, r.width(), kRamp), horizontal);
    }

    if (showHisto_) {
        const int peak = std::max(1, *std::max_element(histo_.begin(), histo_.end()));
        QPainterPath area;
        area.moveTo(r.bottomLeft());
        for (int i = 0; i < 256; ++i) {
            const qreal x = r.left() + (i + 0.5) * r.width() / 256.0;
            area.lineTo(x, r.bottom() - double(histo_[i]) / peak * r.height());
        }
        area.lineTo(r.bottomRight());
        area.closeSubpath();
        p.fillPath(area, Theme::tokenColor(QStringLiteral("graph-fill")));
    }

    p.setPen(QPen(Theme::tokenColor(QStringLiteral("border-input")), 1));
    for (int i = 1; i < 4; ++i) {
        const qreal x = r.left() + i * r.width() / 4.0;
        const qreal y = r.top() + i * r.height() / 4.0;
        p.drawLine(QPointF(x, r.top()), QPointF(x, r.bottom()));
        p.drawLine(QPointF(r.left(), y), QPointF(r.right(), y));
    }

    p.setRenderHint(QPainter::Antialiasing);
    if (showBaseline_) {
        p.setPen(QPen(Theme::tokenColor(QStringLiteral("graph-axis")), 1));
        p.drawLine(r.bottomLeft(), r.topRight());
    }

    QPainterPath curve;
    curve.moveTo(toWidget(QPointF(0.0, curve_[0])));
    for (int i = 1; i < 256; ++i) {
        curve.lineTo(toWidget(QPointF(i / 255.0, curve_[i])));
    }
    p.setPen(QPen(Theme::tokenColor(QStringLiteral("content-body")), 1.6));
    p.setBrush(Qt::NoBrush);
    p.drawPath(curve);

    // Hollow squares; the selected one filled.
    for (int i = 0; i < points_.size(); ++i) {
        const QPointF c = toWidget(points_[i]);
        const qreal half = i == selected_ ? 4.0 : 3.0;
        p.setPen(QPen(Theme::tokenColor(QStringLiteral("content-body")), 1));
        p.setBrush(i == selected_ ? Theme::tokenColor(QStringLiteral("content-body")) : Theme::tokenColor(QStringLiteral("surface-input")));
        p.drawRect(QRectF(c.x() - half, c.y() - half, 2 * half, 2 * half));
    }

    p.setRenderHint(QPainter::Antialiasing, false);
    p.setPen(QPen(Theme::tokenColor(QStringLiteral("border-input")), 1));
    p.setBrush(Qt::NoBrush);
    p.drawRect(r.adjusted(-0.5, -0.5, 0.5, 0.5));
}

void CurveWidget::mousePressEvent(QMouseEvent* event)
{
    dragging_ = -1;
    for (int i = 0; i < points_.size(); ++i) {
        if ((event->position() - toWidget(points_[i])).manhattanLength() < 10) {
            if (event->modifiers() & Qt::ControlModifier) {
                removeAt(i);
                return;
            }
            dragging_ = i;
            select(i);
            return;
        }
    }
    if (points_.size() >= kMaxPoints) {
        return;
    }
    QPointF pos = fromWidget(event->position());
    pos = QPointF(qBound(0.0, pos.x(), 1.0), qBound(0.0, pos.y(), 1.0));
    points_.append(pos);
    interpolate();
    dragging_ = int(points_.indexOf(pos));
    select(dragging_);
    emit curveChanged();
}

void CurveWidget::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ < 0) {
        return;
    }
    QPointF pos = fromWidget(event->position());
    pos = QPointF(qBound(0.0, pos.x(), 1.0), qBound(0.0, pos.y(), 1.0));
    if (dragging_ == 0) {
        pos.setX(points_.first().x());
    } else if (dragging_ == points_.size() - 1) {
        pos.setX(points_.last().x());
    }
    points_[dragging_] = pos;
    interpolate();
    dragging_ = int(points_.indexOf(pos));
    select(dragging_);
    emit curveChanged();
}

void CurveWidget::mouseReleaseEvent(QMouseEvent* event)
{
    if (dragging_ > 0 && dragging_ < points_.size() - 1) {
        const QPointF pos = fromWidget(event->position());
        if (pos.x() < -0.05 || pos.x() > 1.05 || pos.y() < -0.05 || pos.y() > 1.05) {
            removeAt(dragging_);
        }
    }
    dragging_ = -1;
}

void CurveWidget::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) {
        removeAt(selected_);
        return;
    }
    QWidget::keyPressEvent(event);
}

} // namespace pictura
