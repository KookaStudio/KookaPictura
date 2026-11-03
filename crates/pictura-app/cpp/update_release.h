#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QString>

#include <optional>

#include "update_package.h"

namespace pictura {

struct ReleaseAsset {
    QString name;
    QString downloadUrl;
    qint64 size = 0;
    QString sha256;  // lowercase hex, empty when the release has no digest
};

struct ReleaseInfo {
    QString tag;
    QString version;  // tag without a leading `v`
    QString htmlUrl;
    QString body;
    QList<ReleaseAsset> assets;
};

// Parse the GitHub `releases/latest` object. Returns nullopt for malformed
// JSON or a release without the fields the updater needs.
std::optional<ReleaseInfo> parseReleaseJson(const QByteArray& json);

// The asset matching the package method and architecture. When several match,
// the one whose name contains the release version wins.
std::optional<ReleaseAsset> selectReleaseAsset(const ReleaseInfo& release, PackageMethod method,
                                               const QString& architecture);

} // namespace pictura
