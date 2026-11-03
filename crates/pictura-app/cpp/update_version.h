#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>

#include <optional>

namespace pictura {

// A SemVer application version. Build metadata is kept for display but does
// not participate in ordering.
struct AppVersion {
    int major = 0;
    int minor = 0;
    int patch = 0;
    QStringList prerelease;
    QString build;
};

// Parse `MAJOR.MINOR.PATCH[-prerelease][+build]`, with an optional leading
// `v`. Returns nullopt for anything that is not valid SemVer.
std::optional<AppVersion> parseAppVersion(const QString& text);

// SemVer precedence: -1 when `left` is older, 0 when equal, 1 when newer.
// Build metadata is ignored.
int compareAppVersions(const AppVersion& left, const AppVersion& right);

// True when both strings parse and `candidate` has higher precedence.
bool isNewerVersion(const QString& candidate, const QString& current);

} // namespace pictura
