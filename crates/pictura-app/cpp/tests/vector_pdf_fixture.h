#pragma once

#include <QtCore/QBuffer>
#include <QtCore/QFile>
#include <QtCore/QStandardPaths>
#include <QtGui/QPageSize>
#include <QtGui/QPainter>
#include <QtGui/QPdfWriter>

namespace pictura::test {

inline QByteArray vectorPdf(const QColor& secondPage = Qt::blue, bool twoPages = true,
                            const QSizeF& pageSize = QSizeF(100, 50))
{
    QByteArray bytes;
    QBuffer buffer(&bytes);
    if (!buffer.open(QIODevice::WriteOnly)) {
        return {};
    }
    QPdfWriter writer(&buffer);
    writer.setResolution(72);
    writer.setPageSize(QPageSize(pageSize, QPageSize::Point,
                               QStringLiteral("Vector fixture"), QPageSize::ExactMatch));
    writer.setPageMargins(QMarginsF());
    QPainter painter(&writer);
    if (!painter.isActive()) {
        return {};
    }
    painter.fillRect(QRectF(5, 5, 25, 20), Qt::red);
    if (twoPages) {
        if (!writer.newPage()) {
            return {};
        }
        painter.fillRect(QRectF(5, 5, 25, 20), secondPage);
    }
    painter.end();
    return bytes;
}

inline bool writeFixture(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    return !bytes.isEmpty() && file.open(QIODevice::WriteOnly)
        && file.write(bytes) == bytes.size() && file.flush();
}

inline QByteArray fixtureBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

inline bool haveVectorRasterizer()
{
    const QString configured = qEnvironmentVariable("PICTURA_GHOSTSCRIPT");
    return !QStandardPaths::findExecutable(
                configured.isEmpty() ? QStringLiteral("gs") : configured).isEmpty();
}

} // namespace pictura::test
