#include "update_release.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonParseError>
#include <QtCore/QJsonValue>
#include <QtCore/QtGlobal>

namespace pictura {

namespace {

bool isHexDigit(QChar c)
{
    return (c >= QLatin1Char('0') && c <= QLatin1Char('9'))
        || (c >= QLatin1Char('a') && c <= QLatin1Char('f'))
        || (c >= QLatin1Char('A') && c <= QLatin1Char('F'));
}

QString parseDigest(const QJsonValue& value)
{
    const QString digest = value.toString();
    const QString prefix = QStringLiteral("sha256:");
    if (!digest.startsWith(prefix, Qt::CaseInsensitive)) {
        return {};
    }
    const QString hex = digest.mid(prefix.size());
    if (hex.size() != 64) {
        return {};
    }
    for (const QChar c : hex) {
        if (!isHexDigit(c)) {
            return {};
        }
    }
    return hex.toLower();
}

} // namespace

std::optional<ReleaseInfo> parseReleaseJson(const QByteArray& json)
{
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return std::nullopt;
    }

    const QJsonObject root = document.object();
    const QString tag = root.value(QStringLiteral("tag_name")).toString();
    const QString htmlUrl = root.value(QStringLiteral("html_url")).toString();
    const QJsonValue assetsValue = root.value(QStringLiteral("assets"));
    if (tag.isEmpty() || htmlUrl.isEmpty() || !assetsValue.isArray()) {
        return std::nullopt;
    }

    ReleaseInfo release;
    release.tag = tag;
    release.version = tag;
    if (release.version.size() > 1 && release.version.startsWith(QLatin1Char('v'))) {
        release.version.remove(0, 1);
    }
    release.htmlUrl = htmlUrl;
    release.body = root.value(QStringLiteral("body")).toString();

    for (const QJsonValue& value : assetsValue.toArray()) {
        if (!value.isObject()) {
            return std::nullopt;
        }
        const QJsonObject object = value.toObject();
        const QString name = object.value(QStringLiteral("name")).toString();
        const QString url = object.value(QStringLiteral("browser_download_url")).toString();
        if (name.isEmpty() || url.isEmpty()) {
            continue;
        }
        ReleaseAsset asset;
        asset.name = name;
        asset.downloadUrl = url;
        asset.size = object.value(QStringLiteral("size")).toInteger();
        asset.sha256 = parseDigest(object.value(QStringLiteral("digest")));
        release.assets.append(asset);
    }
    return release;
}

std::optional<ReleaseAsset> selectReleaseAsset(const ReleaseInfo& release, PackageMethod method,
                                               const QString& architecture)
{
    const QString suffix = packageAssetSuffix(method, architecture);
    if (suffix.isEmpty()) {
        return std::nullopt;
    }

    const ReleaseAsset* fallback = nullptr;
    for (const ReleaseAsset& asset : release.assets) {
        if (!asset.name.endsWith(suffix, Qt::CaseSensitive)) {
            continue;
        }
        if (!release.version.isEmpty() && asset.name.contains(release.version, Qt::CaseSensitive)) {
            return asset;
        }
        if (!fallback) {
            fallback = &asset;
        }
    }
    if (fallback) {
        return *fallback;
    }
    return std::nullopt;
}

} // namespace pictura
