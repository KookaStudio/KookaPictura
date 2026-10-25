#include "color_picker_math.h"

#include <QtCore/QString>

#include <cmath>

namespace pictura {

namespace {

constexpr int kWebStep = 0x33;

bool isHexDigit(QChar c)
{
    const ushort u = c.unicode();
    return (u >= '0' && u <= '9') || (u >= 'a' && u <= 'f') || (u >= 'A' && u <= 'F');
}

} // namespace

QString hexDigits(QString text)
{
    text = text.trimmed();
    if (text.startsWith(QLatin1Char('#'))) {
        text.remove(0, 1);
    } else if (text.startsWith(QLatin1String("0x"), Qt::CaseInsensitive)) {
        text.remove(0, 2);
    }
    return text;
}

void toHsvPreservingHue(const QColor& c, int& h, int& s, int& v)
{
    const int newHue = c.hue();
    if (newHue >= 0) {
        h = newHue;
    }
    s = c.saturation();
    v = c.value();
}

bool parseHexColor(const QString& text, QColor* color)
{
    QString digits = hexDigits(text);
    if (digits.size() == 3) {
        digits = QString(digits[0]) + digits[0] + digits[1] + digits[1] + digits[2] + digits[2];
    }
    if (digits.size() != 6) {
        return false;
    }
    for (QChar c : digits) {
        if (!isHexDigit(c)) {
            return false;
        }
    }
    const QColor parsed(QStringLiteral("#") + digits);
    if (!parsed.isValid()) {
        return false;
    }
    *color = parsed.toRgb();
    return true;
}

QColor snapToWebColor(const QColor& color)
{
    const auto snap = [](int c) {
        return qBound(0, int(std::lround(c / double(kWebStep)) * kWebStep), 255);
    };
    return QColor(snap(color.red()), snap(color.green()), snap(color.blue()));
}

bool isWebColor(const QColor& color)
{
    return color.red() % kWebStep == 0 && color.green() % kWebStep == 0
        && color.blue() % kWebStep == 0;
}

} // namespace pictura
