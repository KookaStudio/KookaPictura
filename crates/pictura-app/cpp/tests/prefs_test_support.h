#pragma once

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

namespace pictura::test {

inline bool writeProfileFile(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    return QDir().mkpath(QFileInfo(path).absolutePath())
        && file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

inline QByteArray readProfileFile(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray{};
}

inline bool writeProfileJson(const QString& path, const QJsonObject& object)
{
    return writeProfileFile(path, QJsonDocument(object).toJson(QJsonDocument::Compact));
}

inline QJsonObject readProfileJson(const QString& path)
{
    return QJsonDocument::fromJson(readProfileFile(path)).object();
}

} // namespace pictura::test
