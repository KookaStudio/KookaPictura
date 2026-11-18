#include "print_page_layout.h"

#include <QtCore/QJsonArray>
#include <QtGui/QFont>
#include <QtGui/QFontMetricsF>
#include <QtGui/QImage>
#include <QtGui/QPainter>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

constexpr qreal markLength = 10.0;
constexpr qreal gap = 3.0;

void addMark(PrintPageLayout& page, QPainterPath path, const QColor& color = Qt::black,
             QByteArray deviceColor = {})
{
    if (!path.isEmpty() && page.printable.contains(path.boundingRect())) {
        page.marks.append({std::move(path), color, std::move(deviceColor)});
    }
}

void addStroke(PrintPageLayout& page, const QPainterPath& line, qreal width = 0.5)
{
    QPainterPathStroker stroker;
    stroker.setWidth(width);
    stroker.setCapStyle(Qt::FlatCap);
    addMark(page, stroker.createStroke(line));
}

void addLine(QPainterPath& path, QPointF start, QPointF end)
{
    path.moveTo(start);
    path.lineTo(end);
}

void cropAt(PrintPageLayout& page, QPointF origin, int horizontal, int vertical)
{
    QPainterPath crop;
    if (horizontal) {
        const QPointF edge = origin + QPointF(horizontal * gap, 0);
        addLine(crop, edge, edge + QPointF(horizontal * markLength, 0));
    }
    if (vertical) {
        const QPointF edge = origin + QPointF(0, vertical * gap);
        addLine(crop, edge, edge + QPointF(0, vertical * markLength));
    }
    addStroke(page, crop);
}

void registration(PrintPageLayout& page, QPointF center)
{
    QPainterPath target;
    target.addEllipse(center, 3.0, 3.0);
    target.addEllipse(center, 1.0, 1.0);
    addLine(target, center - QPointF(5, 0), center + QPointF(5, 0));
    addLine(target, center - QPointF(0, 5), center + QPointF(0, 5));
    addStroke(page, target);
}

void label(PrintPageLayout& page, const QString& text, qreal baseline)
{
    if (text.isEmpty()) {
        return;
    }
    QFont font(QStringLiteral("Sans Serif"));
    font.setPixelSize(9);
    const QFontMetricsF metrics(font);
    const QString clipped = metrics.elidedText(text.simplified().left(300), Qt::ElideRight,
                                               page.image.width());
    QPainterPath glyphs;
    glyphs.addText(page.image.center().x() - metrics.horizontalAdvance(clipped) / 2,
                   baseline, font, clipped);
    addMark(page, std::move(glyphs));
}

} // namespace

PrintPageLayout layoutPrintPage(const QPageLayout& paper, QSizeF image,
                                const QJsonObject& options, const QString& filename,
                                const QString& description)
{
    const QRectF sheet = paper.fullRect(QPageLayout::Point);
    PrintPageLayout result{sheet.size(), paper.paintRect(QPageLayout::Point), {}, {}, {}};
    const auto enabled = [&](const char* name) { return options.value(QLatin1String(name)).toBool(); };
    const auto number = [&](const char* name) {
        return std::clamp(options.value(QLatin1String(name)).toDouble(), 0.0, 144.0);
    };
    const QJsonArray rgb = options.value(QStringLiteral("background")).toArray();
    if (rgb.size() == 3) {
        result.background = QColor::fromRgbF(std::clamp(rgb.at(0).toDouble() / 255, 0.0, 1.0),
                                             std::clamp(rgb.at(1).toDouble() / 255, 0.0, 1.0),
                                             std::clamp(rgb.at(2).toDouble() / 255, 0.0, 1.0));
    }
    if (image.isEmpty() || result.printable.isEmpty()) {
        return result;
    }
    const bool corner = enabled("cornerCrop");
    const bool center = enabled("centerCrop");
    const bool registerMarks = enabled("registration");
    const bool calibration = enabled("calibration");
    const bool colorBars = enabled("managed") && options.value(QStringLiteral("components")).toInt() == 4;
    const bool showFilename = enabled("filename") && !filename.isEmpty();
    const bool showDescription = enabled("description") && !description.isEmpty();
    const qreal border = number("border");
    const qreal cropMargin = (corner || center) ? gap + markLength + 1 : 0;
    const qreal horizontal = std::max({cropMargin, registerMarks ? 14.0 : 0.0, border});
    const qreal vertical = std::max(cropMargin, border)
        + (showFilename ? 12 : 0) + (showDescription ? 12 : 0) + (calibration ? (colorBars ? 35 : 11) : 0);
    QRectF available = result.printable.adjusted(horizontal, vertical, -horizontal, -vertical);
    constexpr qreal minimumPictureSide = 12.0;
    if (available.width() < minimumPictureSide || available.height() < minimumPictureSide) {
        available = result.printable;
    }
    image.scale(available.size(), Qt::KeepAspectRatio);
    result.image = QRectF(QPointF(), image);
    result.image.moveCenter(available.center());
    const QRectF rect = result.image;
    const qreal bleed = std::min(number("bleed"), std::min(rect.width(), rect.height()) / 2);
    const QRectF trim = rect.adjusted(bleed, bleed, -bleed, -bleed);
    if (border > 0) {
        QPainterPath rim;
        rim.addRect(rect.adjusted(-border, -border, border, border));
        rim.addRect(rect);
        addMark(result, rim);
    }
    if (corner) {
        cropAt(result, trim.topLeft(), -1, -1);
        cropAt(result, trim.topRight(), 1, -1);
        cropAt(result, trim.bottomLeft(), -1, 1);
        cropAt(result, trim.bottomRight(), 1, 1);
    }
    if (center) {
        cropAt(result, QPointF(trim.center().x(), trim.top()), 0, -1);
        cropAt(result, QPointF(trim.center().x(), trim.bottom()), 0, 1);
        cropAt(result, QPointF(trim.left(), trim.center().y()), -1, 0);
        cropAt(result, QPointF(trim.right(), trim.center().y()), 1, 0);
    }
    if (registerMarks) {
        registration(result, QPointF(rect.left() - 8, rect.top() + rect.height() / 3));
        registration(result, QPointF(rect.right() + 8, rect.top() + rect.height() * 2 / 3));
    }
    if (showFilename) {
        label(result, filename, rect.top() - cropMargin - 3);
    }
    if (showDescription) {
        label(result, description, rect.bottom() + cropMargin + 10);
    }
    if (calibration) {
        const qreal width = std::min(88.0, rect.width());
        const qreal step = width / 11;
        const qreal top = rect.bottom() + cropMargin + (showDescription ? 14 : 2);
        for (int plate = 0; plate < (colorBars ? 4 : 1); ++plate) {
            for (int i = 0; i < 11; ++i) {
                QPainterPath patch;
                patch.addRect(rect.center().x() - width / 2 + i * step, top + 8 * plate, step, 6);
                const int ink = qRound(255.0 * i / 10);
                if (colorBars) {
                    QByteArray cmyk(4, '\0');
                    cmyk[plate] = char(ink);
                    const auto color = QColor::fromCmyk(plate == 0 ? ink : 0, plate == 1 ? ink : 0,
                                                        plate == 2 ? ink : 0, plate == 3 ? ink : 0);
                    addMark(result, patch, color, cmyk);
                } else {
                    addMark(result, patch, QColor(255 - ink, 255 - ink, 255 - ink));
                }
            }
        }
    }
    return result;
}

void paintPrintPage(QPainter& painter, const PrintPageLayout& page, const QImage& image,
                    const QColor& paper)
{
    painter.fillRect(QRectF(QPointF(), page.paper), paper);
    if (page.background.isValid()) {
        painter.fillRect(page.printable, page.background);
    }
    painter.setRenderHint(QPainter::SmoothPixmapTransform);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.drawImage(page.image, image);
    for (const PrintMark& mark : page.marks) {
        painter.fillPath(mark.path, mark.color);
    }
}

QString printPathOperators(const QPainterPath& path, double height)
{
    const auto point = [height](QPainterPath::Element element) {
        return QString::number(element.x, 'f', 4) + QLatin1Char(' ')
            + QString::number(height - element.y, 'f', 4);
    };
    QString result;
    for (int index = 0; index < path.elementCount(); ++index) {
        const auto element = path.elementAt(index);
        if (element.isMoveTo()) {
            result += point(element) + QStringLiteral(" m\n");
        } else if (element.isLineTo()) {
            result += point(element) + QStringLiteral(" l\n");
        } else if (element.type == QPainterPath::CurveToElement && index + 2 < path.elementCount()) {
            result += point(element) + QLatin1Char(' ') + point(path.elementAt(index + 1))
                + QLatin1Char(' ') + point(path.elementAt(index + 2)) + QStringLiteral(" c\n");
            index += 2;
        }
    }
    return result + (path.fillRule() == Qt::OddEvenFill ? QStringLiteral("f*\n") : QStringLiteral("f\n"));
}

} // namespace pictura
