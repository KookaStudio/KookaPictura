#include "blend_if_slider.h"

#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <algorithm>
#include <cmath>
#include <iterator>

namespace pictura {

BlendIfSlider::BlendIfSlider(QWidget* parent) : QWidget(parent)
{
    setMinimumWidth(180);
    setFixedHeight(36);
    setToolTip(tr("Drag a handle; hold Alt to move one half."));
}

QSize BlendIfSlider::sizeHint() const { return {260, 36}; }

void BlendIfSlider::setRange(int black, int white)
{
    values_[0] = (black >> 8) & 255;
    values_[1] = black & 255;
    values_[2] = (white >> 8) & 255;
    values_[3] = white & 255;
    update();
}

int BlendIfSlider::position(int value) const
{
    return 8 + qRound(value * (width() - 16) / 255.0);
}

int BlendIfSlider::valueAt(int x) const
{
    return std::clamp(qRound((x - 8) * 255.0 / std::max(1, width() - 16)), 0, 255);
}

void BlendIfSlider::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    QLinearGradient ramp(position(0), 0, position(255), 0);
    ramp.setColorAt(0, Qt::black);
    ramp.setColorAt(1, Qt::white);
    painter.fillRect(QRect(position(0), 3, position(255) - position(0), 13), ramp);
    painter.setPen(palette().color(QPalette::Text));
    painter.drawRect(QRect(position(0), 3, position(255) - position(0), 13));
    for (int i = 0; i < 4; ++i) {
        const int x = position(values_[i]);
        const int direction = i % 2 == 0 ? -1 : 1;
        QPolygon triangle;
        triangle << QPoint(x, 17) << QPoint(x, 29) << QPoint(x + direction * 7, 29);
        painter.setBrush(i < 2 ? Qt::black : Qt::white);
        painter.drawPolygon(triangle);
    }
}

void BlendIfSlider::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) return;
    const int x = event->position().toPoint().x();
    int best = 100000;
    for (int i = 0; i < 4; ++i) {
        const int offset = i % 2 == 0 ? -3 : 3;
        const int distance = std::abs(position(values_[i]) + offset - x);
        if (distance < best) { best = distance; active_ = i; }
    }
    if (best > 12) { active_ = -1; return; }
    std::copy(std::begin(values_), std::end(values_), std::begin(original_));
    pressValue_ = valueAt(x);
    split_ = event->modifiers().testFlag(Qt::AltModifier);
    event->accept();
}

void BlendIfSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (active_ < 0) return;
    // Invalid stored words are left untouched until the user actually drags.
    int ordered[4] = {original_[0], original_[1], original_[2], original_[3]};
    std::sort(std::begin(ordered), std::end(ordered));
    const int delta = valueAt(event->position().toPoint().x()) - pressValue_;
    if (delta == 0) {
        if (!std::equal(std::begin(values_), std::end(values_), std::begin(original_))) {
            std::copy(std::begin(original_), std::end(original_), std::begin(values_));
            update();
            emit rangeChanged(blackWord(), whiteWord());
        }
        return;
    }
    std::copy(std::begin(ordered), std::end(ordered), std::begin(values_));
    if (split_) {
        const int lo = active_ == 0 ? 0 : values_[active_ - 1];
        const int hi = active_ == 3 ? 255 : values_[active_ + 1];
        values_[active_] = std::clamp(ordered[active_] + delta, lo, hi);
    } else {
        const int first = active_ < 2 ? 0 : 2;
        const int lo = first == 0 ? -ordered[0] : ordered[1] - ordered[2];
        const int hi = first == 0 ? ordered[2] - ordered[1] : 255 - ordered[3];
        const int bounded = std::clamp(delta, lo, hi);
        values_[first] += bounded;
        values_[first + 1] += bounded;
    }
    update();
    emit rangeChanged(blackWord(), whiteWord());
}

void BlendIfSlider::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) active_ = -1;
}

} // namespace pictura

#include "moc_blend_if_slider.cpp"
