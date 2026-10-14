#include "spectrum_bar.h"

#include "theme.h"

#include <QtGui/QColor>
#include <QtGui/QLinearGradient>
#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>

namespace pictura {

SpectrumBar::SpectrumBar(QWidget* parent)
    : QWidget(parent)
{
    setFixedHeight(14);
    setMinimumWidth(200);
}

void SpectrumBar::setHueShift(int degrees)
{
    hueShift_ = degrees;
    update();
}

void SpectrumBar::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const int w = width();
    const int h = height();

    QLinearGradient grad(0, 0, w, 0);
    const int stops = 7;
    for (int i = 0; i <= stops; ++i) {
        const qreal pos = static_cast<qreal>(i) / stops;
        int hue = static_cast<int>(pos * 360 + hueShift_) % 360;
        if (hue < 0) {
            hue += 360;
        }
        grad.setColorAt(pos, QColor::fromHsv(hue, 255, 255));
    }
    p.fillRect(0, 0, w, h, grad);

    p.setPen(QPen(Theme::tokenColor(QStringLiteral("inputBorder")), 1));
    p.drawRect(0, 0, w - 1, h - 1);
}

} // namespace pictura
