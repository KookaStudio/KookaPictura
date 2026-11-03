#include "update_version.h"

#include <QtCore/QChar>
#include <QtCore/QtGlobal>

#include <algorithm>
#include <limits>

namespace pictura {

namespace {

bool isAsciiDigit(QChar c)
{
    const char16_t value = c.unicode();
    return value >= u'0' && value <= u'9';
}

bool isAsciiAlpha(QChar c)
{
    const char16_t value = c.unicode();
    return (value >= u'a' && value <= u'z') || (value >= u'A' && value <= u'Z');
}

bool isIdentifierChar(QChar c)
{
    return isAsciiAlpha(c) || isAsciiDigit(c) || c.unicode() == u'-';
}

bool isNumericIdentifier(const QString& text)
{
    if (text.isEmpty()) {
        return false;
    }
    for (const QChar c : text) {
        if (!isAsciiDigit(c)) {
            return false;
        }
    }
    return true;
}

bool parseCoreNumber(const QString& text, int* value)
{
    if (text.isEmpty() || (text.size() > 1 && text.at(0).unicode() == u'0')) {
        return false;
    }
    qlonglong number = 0;
    for (const QChar c : text) {
        if (!isAsciiDigit(c)) {
            return false;
        }
        number = number * 10 + (c.unicode() - u'0');
        if (number > std::numeric_limits<int>::max()) {
            return false;
        }
    }
    *value = int(number);
    return true;
}

bool parseIdentifiers(const QString& text, QStringList* identifiers, bool prerelease)
{
    if (text.isEmpty()) {
        return false;
    }
    const QStringList parts = text.split(QLatin1Char('.'));
    for (const QString& part : parts) {
        if (part.isEmpty()) {
            return false;
        }
        for (const QChar c : part) {
            if (!isIdentifierChar(c)) {
                return false;
            }
        }
        if (prerelease && isNumericIdentifier(part) && part.size() > 1
            && part.at(0).unicode() == u'0') {
            return false;
        }
    }
    *identifiers = parts;
    return true;
}

int compareIdentifiers(const QString& left, const QString& right)
{
    const bool leftNumeric = isNumericIdentifier(left);
    const bool rightNumeric = isNumericIdentifier(right);
    if (leftNumeric && rightNumeric) {
        if (left.size() != right.size()) {
            return left.size() < right.size() ? -1 : 1;
        }
        const int result = left.compare(right, Qt::CaseSensitive);
        return result < 0 ? -1 : (result > 0 ? 1 : 0);
    }
    if (leftNumeric != rightNumeric) {
        return leftNumeric ? -1 : 1;
    }
    const int result = left.compare(right, Qt::CaseSensitive);
    return result < 0 ? -1 : (result > 0 ? 1 : 0);
}

} // namespace

std::optional<AppVersion> parseAppVersion(const QString& text)
{
    QString value = text.trimmed();
    if (value.size() > 1 && (value.at(0).unicode() == u'v' || value.at(0).unicode() == u'V')
        && isAsciiDigit(value.at(1))) {
        value.remove(0, 1);
    }
    if (value.isEmpty()) {
        return std::nullopt;
    }

    AppVersion version;
    const int plus = value.indexOf(QLatin1Char('+'));
    if (plus >= 0) {
        if (plus != value.lastIndexOf(QLatin1Char('+'))) {
            return std::nullopt;
        }
        QStringList build;
        if (!parseIdentifiers(value.mid(plus + 1), &build, false)) {
            return std::nullopt;
        }
        version.build = value.mid(plus + 1);
        value = value.left(plus);
    }

    const int dash = value.indexOf(QLatin1Char('-'));
    if (dash >= 0) {
        if (!parseIdentifiers(value.mid(dash + 1), &version.prerelease, true)) {
            return std::nullopt;
        }
        value = value.left(dash);
    }

    const QStringList core = value.split(QLatin1Char('.'));
    if (core.size() != 3 || !parseCoreNumber(core.at(0), &version.major)
        || !parseCoreNumber(core.at(1), &version.minor)
        || !parseCoreNumber(core.at(2), &version.patch)) {
        return std::nullopt;
    }
    return version;
}

int compareAppVersions(const AppVersion& left, const AppVersion& right)
{
    if (left.major != right.major) {
        return left.major < right.major ? -1 : 1;
    }
    if (left.minor != right.minor) {
        return left.minor < right.minor ? -1 : 1;
    }
    if (left.patch != right.patch) {
        return left.patch < right.patch ? -1 : 1;
    }
    if (left.prerelease.isEmpty() && right.prerelease.isEmpty()) {
        return 0;
    }
    if (left.prerelease.isEmpty()) {
        return 1;
    }
    if (right.prerelease.isEmpty()) {
        return -1;
    }
    const int shared = std::min(left.prerelease.size(), right.prerelease.size());
    for (int i = 0; i < shared; ++i) {
        const int result = compareIdentifiers(left.prerelease.at(i), right.prerelease.at(i));
        if (result != 0) {
            return result;
        }
    }
    if (left.prerelease.size() == right.prerelease.size()) {
        return 0;
    }
    return left.prerelease.size() < right.prerelease.size() ? -1 : 1;
}

bool isNewerVersion(const QString& candidate, const QString& current)
{
    const std::optional<AppVersion> candidateVersion = parseAppVersion(candidate);
    const std::optional<AppVersion> currentVersion = parseAppVersion(current);
    if (!candidateVersion || !currentVersion) {
        return false;
    }
    return compareAppVersions(*candidateVersion, *currentVersion) > 0;
}

} // namespace pictura
