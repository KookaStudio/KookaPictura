#pragma once

// The Blur Gallery Bokeh tab's Light Range control.

#include <QtCore/QRectF>
#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QWidget>

#include <algorithm>
#include <cstdlib>
#include <functional>

namespace pictura {

// The Light Range control: a black-to-white bar with a low and a high handle,
// 0..255. The dialog drives it and reads it back; `changed` fires on a drag.
class LightRangeSlider : public QWidget {
public:
    explicit LightRangeSlider(QWidget* parent = nullptr) : QWidget(parent)
    {
        setObjectName(QStringLiteral("bokehRange"));
        setMinimumHeight(20);
        setCursor(Qt::PointingHandCursor);
    }

    int low() const { return low_; }
    int high() const { return high_; }

    void setRange(int low, int high)
    {
        low_ = std::clamp(low, 0, 255);
        high_ = std::clamp(high, 0, 255);
        if (low_ > high_) {
            std::swap(low_, high_);
        }
        update();
    }

    std::function<void(int, int)> changed;

    QSize sizeHint() const override { return {200, 20}; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing, true);
        const QRectF groove = grooveRect();
        QLinearGradient ramp(groove.topLeft(), groove.topRight());
        ramp.setColorAt(0.0, Qt::black);
        ramp.setColorAt(1.0, Qt::white);
        painter.setPen(QPen(QColor(90, 90, 90), 1));
        painter.setBrush(ramp);
        painter.drawRect(groove);
        const auto handle = [&](int value, const QColor& color) {
            const qreal x = handleX(value);
            QPainterPath path;
            path.moveTo(x, groove.bottom() + 1.0);
            path.lineTo(x - 5.0, groove.bottom() + 7.0);
            path.lineTo(x + 5.0, groove.bottom() + 7.0);
            path.closeSubpath();
            painter.setBrush(isEnabled() ? color : color.darker(160));
            painter.setPen(QPen(QColor(60, 60, 60), 1));
            painter.drawPath(path);
        };
        handle(low_, Qt::black);
        handle(high_, Qt::white);
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (!isEnabled()) {
            return;
        }
        const int x = int(event->position().x());
        dragging_ = std::abs(x - handleX(low_)) <= std::abs(x - handleX(high_)) ? 0 : 1;
        mouseMoveEvent(event);
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (dragging_ < 0) {
            return;
        }
        const int value = valueAt(int(event->position().x()));
        if (dragging_ == 0) {
            low_ = std::min(value, high_);
        } else {
            high_ = std::max(value, low_);
        }
        update();
        if (changed) {
            changed(low_, high_);
        }
    }

    void mouseReleaseEvent(QMouseEvent*) override { dragging_ = -1; }

private:
    QRectF grooveRect() const { return QRectF(8.0, 1.0, width() - 16.0, 8.0); }
    int handleX(int value) const { return 8 + value * (width() - 16) / 255; }
    int valueAt(int x) const
    {
        return std::clamp((x - 8) * 255 / std::max(1, width() - 16), 0, 255);
    }

    int low_ = 40;
    int high_ = 118;
    int dragging_ = -1;
};

} // namespace pictura
