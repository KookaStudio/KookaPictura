#include "acr_settings.h"

namespace pictura {

namespace {

QByteArray escape(const QByteArray& bytes)
{
    static const char* kHex = "0123456789abcdef";
    QByteArray out;
    out.reserve(bytes.size());
    for (int i = 0; i < bytes.size(); ++i) {
        const unsigned char c = static_cast<unsigned char>(bytes.at(i));
        switch (c) {
        case '\\':
            out.append("\\\\");
            break;
        case ';':
            out.append("\\;");
            break;
        case '=':
            out.append("\\=");
            break;
        default:
            if (c >= 0x20 && c <= 0x7e) {
                out.append(static_cast<char>(c));
            } else {
                out.append("\\x");
                out.append(kHex[c >> 4]);
                out.append(kHex[c & 0x0f]);
            }
        }
    }
    return out;
}

int hexValue(char c)
{
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return 0;
}

QByteArray unescape(const QByteArray& text)
{
    QByteArray out;
    out.reserve(text.size());
    int i = 0;
    while (i < text.size()) {
        if (text.at(i) == '\\' && i + 1 < text.size()) {
            const char next = text.at(i + 1);
            if (next == '\\' || next == ';' || next == '=') {
                out.append(next);
                i += 2;
            } else if (next == 'x' && i + 3 < text.size()) {
                out.append(static_cast<char>(hexValue(text.at(i + 2)) * 16
                                             + hexValue(text.at(i + 3))));
                i += 4;
            } else {
                out.append('\\');
                out.append(next);
                i += 2;
            }
        } else {
            out.append(text.at(i));
            ++i;
        }
    }
    return out;
}

QList<QByteArray> splitEscaped(const QByteArray& text, char delim)
{
    QList<QByteArray> parts;
    int start = 0;
    int i = 0;
    while (i < text.size()) {
        if (text.at(i) == '\\') {
            i += 2;
            continue;
        }
        if (text.at(i) == delim) {
            parts.append(text.mid(start, i - start));
            start = i + 1;
        }
        ++i;
    }
    parts.append(text.mid(start));
    return parts;
}

int findUnescaped(const QByteArray& text, char delim)
{
    int i = 0;
    while (i < text.size()) {
        if (text.at(i) == '\\') {
            i += 2;
            continue;
        }
        if (text.at(i) == delim) {
            return i;
        }
        ++i;
    }
    return -1;
}

} // namespace

QByteArray AcrSettings::raw(const QByteArray& key, const QByteArray& fallback) const
{
    return values_.value(key, fallback);
}

void AcrSettings::setRaw(const QByteArray& key, const QByteArray& value)
{
    values_.insert(key, value);
}

int AcrSettings::intValue(const QByteArray& key, int fallback) const
{
    bool ok = false;
    const int value = raw(key).toInt(&ok);
    return ok ? value : fallback;
}

double AcrSettings::doubleValue(const QByteArray& key, double fallback) const
{
    bool ok = false;
    const double value = raw(key).toDouble(&ok);
    return ok ? value : fallback;
}

bool AcrSettings::boolValue(const QByteArray& key, bool fallback) const
{
    const QByteArray value = raw(key);
    if (value.isEmpty()) {
        return fallback;
    }
    return value.toInt() != 0;
}

QString AcrSettings::textValue(const QByteArray& key, const QString& fallback) const
{
    const QByteArray value = raw(key);
    return value.isEmpty() ? fallback : QString::fromUtf8(value);
}

QList<int> AcrSettings::curveValue(const QByteArray& key) const
{
    QList<int> points;
    const QByteArray value = raw(key);
    if (value.isEmpty()) {
        return points;
    }
    for (const QByteArray& token : value.split(',')) {
        if (!token.isEmpty()) {
            points.append(token.toInt());
        }
    }
    return points;
}

void AcrSettings::setInt(const QByteArray& key, int value)
{
    setRaw(key, QByteArray::number(value));
}

void AcrSettings::setDouble(const QByteArray& key, double value)
{
    setRaw(key, QByteArray::number(value, 'g', 17));
}

void AcrSettings::setBool(const QByteArray& key, bool value)
{
    setRaw(key, value ? QByteArrayLiteral("1") : QByteArrayLiteral("0"));
}

void AcrSettings::setText(const QByteArray& key, const QString& value)
{
    setRaw(key, value.toUtf8());
}

void AcrSettings::setCurve(const QByteArray& key, const QList<int>& value)
{
    QByteArray joined;
    for (int i = 0; i < value.size(); ++i) {
        if (i > 0) {
            joined.append(',');
        }
        joined.append(QByteArray::number(value.at(i)));
    }
    setRaw(key, joined);
}

QString AcrSettings::encode() const
{
    QByteArray out;
    bool first = true;
    for (auto it = values_.cbegin(); it != values_.cend(); ++it) {
        if (!first) {
            out.append(';');
        }
        first = false;
        out.append(escape(it.key()));
        out.append('=');
        out.append(escape(it.value()));
    }
    return QString::fromUtf8(out);
}

AcrSettings AcrSettings::decode(const QString& text)
{
    AcrSettings settings;
    const QByteArray bytes = text.toUtf8();
    for (const QByteArray& segment : splitEscaped(bytes, ';')) {
        const int eq = findUnescaped(segment, '=');
        if (eq < 0) {
            continue;
        }
        settings.setRaw(unescape(segment.left(eq)), unescape(segment.mid(eq + 1)));
    }
    return settings;
}

} // namespace pictura
