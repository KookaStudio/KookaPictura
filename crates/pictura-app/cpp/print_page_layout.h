#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QJsonObject>
#include <QtGui/QColor>
#include <QtGui/QPainterPath>
#include <QtGui/QPageLayout>

class QImage;
class QPainter;

namespace pictura {

struct PrintMark {
    QPainterPath path;
    QColor color;
    QByteArray deviceColor;
};

struct PrintPageLayout {
    QSizeF paper;
    QRectF printable;
    QRectF image;
    QColor background;
    QList<PrintMark> marks;
};

PrintPageLayout layoutPrintPage(const QPageLayout& paper, QSizeF image,
                                const QJsonObject& options, const QString& filename,
                                const QString& description);
void paintPrintPage(QPainter& painter, const PrintPageLayout& page, const QImage& image,
                    const QColor& paper);
QString printPathOperators(const QPainterPath& path, double paperHeight);

} // namespace pictura
