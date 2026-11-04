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

namespace {

QCursor renderCursor(const QString& id, int hotX, int hotY, double degrees, qreal dpr)
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

} // namespace

QCursor cursor(const QString& id, int hotX, int hotY, double degrees)
{
    // A hover asks for its cursor on every pointer move. Reusing one QCursor
    // per key keeps its pixmap's cache key, so xcb reuses the X cursor it made
    // rather than creating one per move. Leaked so no pixmap is destroyed after
    // the application in static destruction.
    static auto* cache = new QHash<QString, QCursor>();
    const qreal dpr = qApp ? qApp->devicePixelRatio() : 1.0;
    const QString key = QStringLiteral("%1|%2|%3|%4|%5")
                            .arg(id)
                            .arg(hotX)
                            .arg(hotY)
                            .arg(degrees, 0, 'g', 17)
                            .arg(dpr, 0, 'g', 17);
    auto it = cache->constFind(key);
    if (it == cache->constEnd()) {
        it = cache->insert(key, renderCursor(id, hotX, hotY, degrees, dpr));
    }
    return *it;
}

QCursor rotateCursor(const QPointF& centre, const QPointF& pointer)
{
    const QLineF out(centre, pointer);
    const double heading = out.length() > 1e-6 ? -out.angle() : -45.0;
    const int step = int(std::lround((heading + 45.0) / 5.0)) % 72;
    return cursor(QStringLiteral("cursor.rotate"), 12, 12, (step < 0 ? step + 72 : step) * 5.0);
}

} // namespace pictura
