#include "curve_widget.h"

#include "theme.h"

#include <QtCore/QRectF>
#include <QtCore/QStringList>
#include <QtCore/QSignalBlocker>
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

// Piecewise linear between the control points, matching the engine contour LUT.
void linearInterpolate(const QVector<QPointF>& pts, float out[256])
{
    const int n = pts.size();
    if (n == 0) {
        for (int i = 0; i < 256; ++i) {
            out[i] = i / 255.0f;
        }
        return;
    }
    for (int ix = 0; ix < 256; ++ix) {
        const double x = ix / 255.0;
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
        const double h = pts[seg + 1].x() - pts[seg].x();
        if (h < 1e-12) {
            out[ix] = qBound(0.0f, static_cast<float>(pts[seg].y()), 1.0f);
            continue;
        }
        const double t = (x - pts[seg].x()) / h;
        out[ix] = qBound(
            0.0f,
            static_cast<float>(pts[seg].y() + (pts[seg + 1].y() - pts[seg].y()) * t),
            1.0f);
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

void CurveWidget::useSquareLayout()
{
    QSizePolicy policy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    policy.setHeightForWidth(true);
    setSizePolicy(policy);
    setMinimumSize(96, 96);
}

int CurveWidget::heightForWidth(int width) const
{
    return sizePolicy().hasHeightForWidth() ? width : QWidget::heightForWidth(width);
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

namespace {

QVector<QPointF> pointsFromText(const QString& text)
{
    QVector<QPointF> points;
    for (const QString& pair : text.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        const QStringList xy = pair.split(QLatin1Char(','));
        if (xy.size() == 2) {
            points.append(QPointF(xy[0].toInt() / 255.0, xy[1].toInt() / 255.0));
        }
    }
    return points;
}

} // namespace

void CurveWidget::setPointsText(const QString& text)
{
    const QSignalBlocker block(this);
    auto points = pointsFromText(text);
    pencil_ = points.size() == 256;
    setPoints(points.isEmpty() ? QVector<QPointF>{{0, 0}, {1, 1}} : points);
}

void CurveWidget::setGraphOptions(bool detailed, bool guides, bool reversed)
{
    detailedGrid_ = detailed;
    guides_ = guides;
    reversed_ = reversed;
    update();
}

void CurveWidget::setOverlays(const QVector<QPair<QColor, QString>>& overlays)
{
    overlays_.clear();
    for (const auto& [color, text] : overlays) {
        const auto points = pointsFromText(text);
        QVector<float> samples(256);
        if (points.size() == 256) {
            for (int x = 0; x < 256; ++x) {
                samples[x] = float(points[x].y());
            }
        } else {
            splineInterpolate(points, samples.data());
        }
        overlays_.append({color, samples});
    }
    update();
}

bool CurveWidget::setPencilEnabled(bool enabled)
{
    if (!enabled && hasDenseCurve()) {
        QVector<QPointF> sparse;
        for (int i = 0; i < kMaxPoints; ++i) {
            sparse.append(points_[(i * 255 + 6) / 13]);
        }
        pencil_ = false;
        setPoints(sparse);
        return true;
    }
    pencil_ = enabled;
    select(-1);
    return false;
}

void CurveWidget::drawPencilTo(QPointF position)
{
    if (!hasDenseCurve()) {
        points_.clear();
        for (int i = 0; i < 256; ++i) {
            points_.append({i / 255.0, outputAt(i) / 255.0});
        }
    }
    const QPointF point = fromWidget(position);
    const int x = qBound(0, qRound(point.x() * 255.0), 255);
    const int y = qBound(0, qRound(point.y() * 255.0), 255);
    if (pencilLast_ >= 0 && pencilLast_ != x) {
        const double first = points_[pencilLast_].y();
        for (int i = std::min(pencilLast_, x); i <= std::max(pencilLast_, x); ++i) {
            const double t = double(i - pencilLast_) / (x - pencilLast_);
            points_[i].setY(qRound((1.0 - t) * first * 255.0 + t * y) / 255.0);
        }
    }
    points_[x].setY(y / 255.0);
    pencilLast_ = x;
    interpolate();
    update();
    emit curveChanged();
}

void CurveWidget::smoothPencil()
{
    if (!hasDenseCurve()) {
        return;
    }
    auto smooth = points_;
    for (int x = 1; x < 255; ++x) {
        const double v = (points_[x - 1].y() + 2 * points_[x].y() + points_[x + 1].y()) / 4;
        smooth[x].setY(qRound(v * 255.0) / 255.0);
    }
    if (smooth != points_) {
        setPoints(smooth);
    }
}

int CurveWidget::outputAt(int input) const
{
    return qRound(curve_[qBound(0, input, 255)] * 255.0f);
}

bool CurveWidget::selectTone(int input)
{
    if (pencil_ || input < 0 || input > 255 || points_.isEmpty()) {
        return false;
    }
    int index = 0;
    while (index < points_.size() && qRound(points_[index].x() * 255.0) < input) {
        ++index;
    }
    if (index == points_.size() || qRound(points_[index].x() * 255.0) != input) {
        if (index == 0 || index == points_.size() || points_.size() >= kMaxPoints) {
            return false;
        }
        points_.insert(index, QPointF(input / 255.0, outputAt(input) / 255.0));
        interpolate();
        emit curveChanged();
    }
    select(index);
    return true;
}

void CurveWidget::setEndpoint(bool white, int input, bool resetOutput)
{
    if (pencil_ || points_.size() < 2) {
        return;
    }
    select(white ? int(points_.size()) - 1 : 0);
    const int output = resetOutput ? (white ? 255 : 0) : qRound(points_[selected_].y() * 255);
    moveSelected(input, output);
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
    if (hasDenseCurve()) {
        for (int x = 0; x < 256; ++x) {
            curve_[x] = float(points_[x].y());
        }
    } else if (linear_) {
        linearInterpolate(points_, curve_);
    } else {
        splineInterpolate(points_, curve_);
    }
}

void CurveWidget::setLinearInterpolation(bool linear)
{
    linear_ = linear;
    interpolate();
    update();
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
    const auto inputAt = [this](int i) { return qRound(points_[i].x() * 255.0); };
    int lower = selected_ == 0 ? 0 : inputAt(selected_ - 1) + 1;
    int upper = selected_ == last ? 255 : inputAt(selected_ + 1) - 1;
    if (!secondaryControls_ && (selected_ == 0 || selected_ == last)) {
        lower = upper = inputAt(selected_);
    }
    if (lower > upper) {
        return;
    }
    const QPointF next(qBound(lower, input, upper) / 255.0, qBound(0, output, 255) / 255.0);
    if (points_[selected_] == next) {
        return;
    }
    points_[selected_] = next;
    interpolate();
    update();
    emit curveChanged();
}

void CurveWidget::selectFirstPoint()
{
    if (!points_.isEmpty()) select(0);
}

QMargins CurveWidget::plotInsets() const
{
    // An endpoint's square overhangs the face by 4 px.
    constexpr int inset = 5;
    const int ramp = showRamps_ ? int(kRamp + kGap) : inset;
    return {ramp, inset, inset, ramp};
}

QRectF CurveWidget::plot() const
{
    const auto insets = plotInsets();
    const int side = std::max(1, std::min(width() - insets.left() - insets.right(),
                                          height() - insets.top() - insets.bottom()));
    return QRectF(insets.left(), insets.top(), side, side);
}

QPointF CurveWidget::toWidget(QPointF p) const
{
    const QRectF r = plot();
    if (reversed_) {
        p = QPointF(1.0 - p.x(), 1.0 - p.y());
    }
    return {r.left() + p.x() * r.width(), r.top() + (1.0 - p.y()) * r.height()};
}

QPointF CurveWidget::fromWidget(QPointF p) const
{
    const QRectF r = plot();
    const QPointF unit((p.x() - r.left()) / r.width(), 1.0 - (p.y() - r.top()) / r.height());
    return reversed_ ? QPointF(1.0 - unit.x(), 1.0 - unit.y()) : unit;
}

void CurveWidget::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const QRectF r = plot();
    p.fillRect(r, Theme::tokenColor(QStringLiteral("surface-input")));

    if (showRamps_) {
        QLinearGradient vertical(r.left(), r.bottom(), r.left(), r.top());
        vertical.setColorAt(0.0, reversed_ ? Qt::white : Qt::black);
        vertical.setColorAt(1.0, reversed_ ? Qt::black : Qt::white);
        p.fillRect(QRectF(0.0, r.top(), kRamp, r.height()), vertical);
        QLinearGradient horizontal(r.left(), 0.0, r.right(), 0.0);
        horizontal.setColorAt(0.0, reversed_ ? Qt::white : Qt::black);
        horizontal.setColorAt(1.0, reversed_ ? Qt::black : Qt::white);
        p.fillRect(QRectF(r.left(), r.bottom() + kGap, r.width(), kRamp), horizontal);
    }

    if (showHisto_) {
        const int peak = std::max(1, *std::max_element(histo_.begin(), histo_.end()));
        QPainterPath area;
        area.moveTo(r.bottomLeft());
        for (int i = 0; i < 256; ++i) {
            const qreal x = r.left() + (i + 0.5) * r.width() / 256.0;
            area.lineTo(x, r.bottom() - double(histo_[reversed_ ? 255 - i : i]) / peak * r.height());
        }
        area.lineTo(r.bottomRight());
        area.closeSubpath();
        p.fillPath(area, Theme::tokenColor(QStringLiteral("graph-fill")));
    }

    p.setPen(QPen(Theme::tokenColor(QStringLiteral("border-input")), 1));
    const int divisions = detailedGrid_ ? 10 : 4;
    for (int i = 1; i < divisions; ++i) {
        const qreal x = r.left() + i * r.width() / divisions;
        const qreal y = r.top() + i * r.height() / divisions;
        p.drawLine(QPointF(x, r.top()), QPointF(x, r.bottom()));
        p.drawLine(QPointF(r.left(), y), QPointF(r.right(), y));
    }

    p.setRenderHint(QPainter::Antialiasing);
    if (showBaseline_) {
        p.setPen(QPen(Theme::tokenColor(QStringLiteral("graph-axis")), 1));
        p.drawLine(r.bottomLeft(), r.topRight());
    }

    const auto drawCurve = [&](const float* samples, const QColor& color, double width) {
        QPainterPath path(toWidget(QPointF(0.0, samples[0])));
        for (int i = 1; i < 256; ++i) {
            path.lineTo(toWidget(QPointF(i / 255.0, samples[i])));
        }
        p.setPen(QPen(color, width));
        p.setBrush(Qt::NoBrush);
        p.drawPath(path);
    };
    for (const auto& [color, samples] : overlays_) {
        drawCurve(samples.constData(), color, 1);
    }
    if (guides_ && dragging_ >= 0 && dragging_ < points_.size()) {
        const QPointF at = toWidget(points_[dragging_]);
        p.setPen(QPen(Theme::tokenColor(QStringLiteral("graph-axis")), 1, Qt::DashLine));
        p.drawLine(QPointF(at.x(), r.top()), QPointF(at.x(), r.bottom()));
        p.drawLine(QPointF(r.left(), at.y()), QPointF(r.right(), at.y()));
    }
    drawCurve(curve_, Theme::tokenColor(QStringLiteral("content-body")), 1.6);

    // Hollow squares; the selected one filled.
    for (int i = 0; !pencil_ && i < points_.size(); ++i) {
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
    if (secondaryControls_ && event->button() != Qt::LeftButton) {
        return;
    }
    if (secondaryControls_ && event->modifiers().testFlag(Qt::AltModifier)) {
        emit gridToggled();
        return;
    }
    dragging_ = -1;
    if (secondaryControls_) {
        gesture_ = true;
        emit gestureStarted();
        if (pencil_) {
            pencilLast_ = -1;
            drawPencilTo(event->position());
            return;
        }
    }
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
    if (points_.size() >= kMaxPoints || !plot().contains(event->position())) {
        return;
    }
    QPointF pos = fromWidget(event->position());
    pos = QPointF(qBound(0.0, pos.x(), 1.0), qBound(0.0, pos.y(), 1.0));
    if (secondaryControls_) {
        const int x = qRound(pos.x() * 255.0);
        if (selectTone(x)) {
            dragging_ = selected_;
            moveSelected(x, qRound(pos.y() * 255.0));
        }
        return;
    }
    if (std::any_of(points_.cbegin(), points_.cend(),
                    [pos](QPointF point) { return point.x() == pos.x(); })) {
        return;
    }
    points_.append(pos);
    interpolate();
    dragging_ = int(points_.indexOf(pos));
    select(dragging_);
    emit curveChanged();
}

void CurveWidget::mouseMoveEvent(QMouseEvent* event)
{
    if (pencil_ && gesture_) {
        drawPencilTo(event->position());
        return;
    }
    if (dragging_ < 0) {
        return;
    }
    QPointF pos = fromWidget(event->position());
    if (secondaryControls_) {
        moveSelected(qBound(0, qRound(pos.x() * 255.0), 255),
                     qBound(0, qRound(pos.y() * 255.0), 255));
        return;
    }
    pos = QPointF(qBound(0.0, pos.x(), 1.0), qBound(0.0, pos.y(), 1.0));
    if (dragging_ == 0) {
        pos.setX(points_.first().x());
    } else if (dragging_ == points_.size() - 1) {
        pos.setX(points_.last().x());
    }
    for (int i = 0; i < points_.size(); ++i) {
        if (i != dragging_ && points_[i].x() == pos.x()) {
            pos.setX(points_[dragging_].x());
            break;
        }
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
    pencilLast_ = -1;
    update();
    if (gesture_) {
        gesture_ = false;
        emit gestureEnded();
    }
}

void CurveWidget::keyPressEvent(QKeyEvent* event)
{
    if (pencil_) {
        QWidget::keyPressEvent(event);
        return;
    }
    if (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) {
        removeAt(selected_);
        return;
    }
    QWidget::keyPressEvent(event);
}

} // namespace pictura

#include "moc_curve_widget.cpp"
