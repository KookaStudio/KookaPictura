#include "icons.h"

#include <QtCore/QFile>
#include <QtCore/QHash>
#include <QtCore/QLineF>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtGui/QCursor>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtSvg/QSvgRenderer>
#include <QtWidgets/QApplication>

#include <cmath>

#include "svg_icon_engine.h"

namespace pictura {

QIcon icon(const QString& id)
{
    const QString svg = QStringLiteral(":/icons/") + id + QStringLiteral(".svg");
    if (QFile::exists(svg)) {
        return QIcon(new SvgIconEngine(svg));
    }
    const QString png = QStringLiteral(":/icons/") + id + QStringLiteral(".png");
    if (QFile::exists(png)) {
        return QIcon(png);
    }
    return QIcon();
}

QIcon icon(const QString& id, const QColor& color)
{
    const QString svg = QStringLiteral(":/icons/") + id + QStringLiteral(".svg");
    if (QFile::exists(svg)) {
        return QIcon(new SvgIconEngine(svg, color));
    }
    return icon(id);
}

QCursor cursor(const QString& id)
{
    return cursor(id, 12, 12);
}

QCursor cursor(const QString& id, int hotX, int hotY)
{
    return cursor(id, hotX, hotY, 0.0);
}

QCursor cursor(const QString& id, int hotX, int hotY, double degrees)
{
    const QString path = QStringLiteral(":/cursors/") + id + QStringLiteral(".svg");
    if (!QFile::exists(path)) {
        return QCursor();
    }

    QSvgRenderer renderer(path);
    if (!renderer.isValid()) {
        return QCursor();
    }

    // ponytail: single-DPR render; add a multi-resolution pixmap set if HiDPI
    // cursors look soft.
    const qreal dpr = qApp ? qApp->devicePixelRatio() : 1.0;
    const int size = qRound(24 * dpr);
    QPixmap pixmap(size, size);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    if (degrees != 0.0) {
        painter.setRenderHint(QPainter::Antialiasing);
        painter.translate(hotX * dpr, hotY * dpr);
        painter.rotate(degrees);
        painter.translate(-hotX * dpr, -hotY * dpr);
    }
    renderer.render(&painter);
    painter.end();
    pixmap.setDevicePixelRatio(dpr);

    return QCursor(pixmap, qRound(hotX * dpr), qRound(hotY * dpr));
}

QCursor rotateCursor(const QPointF& centre, const QPointF& pointer)
{
    static QHash<int, QCursor> cache;
    const QLineF out(centre, pointer);
    const double heading = out.length() > 1e-6 ? -out.angle() : -45.0;
    const int step = int(std::lround((heading + 45.0) / 5.0)) % 72;
    const int key = step < 0 ? step + 72 : step;
    auto it = cache.find(key);
    if (it == cache.end()) {
        it = cache.insert(key, cursor(QStringLiteral("cursor.rotate"), 12, 12, key * 5.0));
    }
    return it.value();
}

} // namespace pictura
