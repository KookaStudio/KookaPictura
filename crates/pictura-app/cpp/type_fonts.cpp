// Host fonts for the Type tools: Qt resolves the family the options bar names,
// and its tables are reassembled into an sfnt the engine shapes and
// rasterizes. ponytail: a variable font's named instance arrives as the
// variable font, so it renders as the default instance.

#include "type_fonts.h"

#include "fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QSet>
#include <QtCore/QtEndian>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtGui/QRawFont>

#include <algorithm>

namespace pictura {

namespace {

// Every table rustybuzz, swash, or ttf-parser reads, outline or bitmap.
const char* const kTables[] = {
    "BASE", "CBDT", "CBLC", "CFF ", "CFF2", "COLR", "CPAL", "GDEF", "GPOS", "GSUB", "HVAR",
    "MVAR", "OS/2", "STAT", "SVG ", "VORG", "VVAR", "ankr", "avar", "cmap", "cvar", "cvt ",
    "feat", "fpgm", "fvar", "gasp", "glyf", "gvar", "hdmx", "head", "hhea", "hmtx", "kern",
    "kerx", "loca", "maxp", "morx", "name", "post", "prep", "sbix", "trak", "vhea", "vmtx",
};

void putU16(QByteArray& out, quint16 v)
{
    v = qToBigEndian(v);
    out.append(reinterpret_cast<const char*>(&v), 2);
}

void putU32(QByteArray& out, quint32 v)
{
    v = qToBigEndian(v);
    out.append(reinterpret_cast<const char*>(&v), 4);
}

quint32 checksum(const QByteArray& data)
{
    quint32 sum = 0;
    for (qsizetype i = 0; i < data.size(); i += 4) {
        quint32 word = 0;
        for (int b = 0; b < 4; ++b) {
            word = (word << 8) | quint8(i + b < data.size() ? data.at(i + b) : 0);
        }
        sum += word;
    }
    return sum;
}

// Family+style faces already handed to the engine (or tried).
QSet<QString>& tried()
{
    static QSet<QString> faces;
    return faces;
}

// Family names already handed to the engine, whatever their style.
QSet<QString>& familiesTried()
{
    static QSet<QString> families;
    return families;
}

} // namespace

QByteArray typeFontBytes(const QString& family, const QString& style)
{
    QFont resolved(family);
    if (!style.isEmpty()) {
        const QStringList styles = QFontDatabase::styles(family);
        const auto match = std::find_if(styles.cbegin(), styles.cend(), [&style](const QString& s) {
            return s.compare(style, Qt::CaseInsensitive) == 0;
        });
        if (match != styles.cend()) {
            resolved.setStyleName(*match);
        } else {
            // A family may name its faces differently from the bridge's style
            // ("Demi" versus "SemiBold"); map the common words rather than
            // silently rendering the regular face.
            const QString lower = style.toLower();
            resolved.setBold(lower.contains(QStringLiteral("bold"))
                             || lower.contains(QStringLiteral("black"))
                             || lower.contains(QStringLiteral("demi"))
                             || lower.contains(QStringLiteral("semibold")));
            resolved.setItalic(lower.contains(QStringLiteral("italic"))
                               || lower.contains(QStringLiteral("oblique")));
            if (lower.contains(QStringLiteral("light"))) {
                resolved.setWeight(QFont::Light);
            }
        }
    }
    QRawFont raw = QRawFont::fromFont(resolved);
    if (!raw.isValid() && !style.isEmpty()) {
        raw = QRawFont::fromFont(QFont(family));
    }
    if (!raw.isValid()) {
        return {};
    }
    QList<std::pair<QByteArray, QByteArray>> tables;
    for (const char* tag : kTables) {
        const QByteArray data = raw.fontTable(tag);
        if (!data.isEmpty()) {
            tables.append({QByteArray(tag, 4), data});
        }
    }
    const auto has = [&tables](const char* tag) {
        return std::any_of(tables.begin(), tables.end(),
                           [tag](const auto& t) { return t.first == QByteArray(tag, 4); });
    };
    if (!has("cmap") || !has("head") || !has("maxp")) {
        return {};
    }
    const quint16 count = quint16(tables.size());
    quint16 power = 1;
    quint16 selector = 0;
    while (power * 2 <= count) {
        power *= 2;
        ++selector;
    }
    QByteArray out;
    putU32(out, has("glyf") || !has("CFF ") ? 0x00010000u : 0x4F54544Fu);
    putU16(out, count);
    putU16(out, quint16(power * 16));
    putU16(out, selector);
    putU16(out, quint16(count * 16 - power * 16));
    quint32 offset = 12 + 16 * quint32(count);
    for (const auto& [tag, data] : tables) {
        out.append(tag);
        putU32(out, checksum(data));
        putU32(out, offset);
        putU32(out, quint32(data.size()));
        offset += quint32((data.size() + 3) & ~qsizetype(3));
    }
    for (const auto& table : tables) {
        out.append(table.second);
        out.append(QByteArray((4 - table.second.size() % 4) % 4, '\0'));
    }
    return out;
}

void registerTypeFont(const QString& family, const QString& style)
{
    const QString key = family + QLatin1Char('|') + style;
    if (family.isEmpty() || tried().contains(key)) {
        return;
    }
    tried().insert(key);
    familiesTried().insert(family);
    const QByteArray bytes = typeFontBytes(family, style);
    if (!bytes.isEmpty()) {
        pictura::type_api::registerFont(family, style, ::rust::Slice<const std::uint8_t>(
                                       reinterpret_cast<const std::uint8_t*>(bytes.constData()),
                                       std::size_t(bytes.size())));
    }
}

void registerTypeFallbacks()
{
    static bool done = false;
    if (done) {
        return;
    }
    done = true;
    // The bundled UI face is always available and covers Greek and Cyrillic,
    // which the metric-compatible Liberation face does not.
    registerBundledFonts();
    registerTypeFont(QString::fromLatin1(kBundledUiFamily));
    const QStringList samples = {
        QStringLiteral("ا"), QStringLiteral("א"), QStringLiteral("अ"),
        QStringLiteral("漢"), QStringLiteral("あ"), QStringLiteral("한"),
        QStringLiteral("ก"),
    };
    QStringList families = QFontDatabase::families();
    families.sort(Qt::CaseInsensitive);
    for (const QString& sample : samples) {
        const uint code = sample.at(0).unicode();
        for (const QString& family : families) {
            const QRawFont raw = QRawFont::fromFont(QFont(family));
            if (raw.isValid() && raw.supportsCharacter(code)) {
                registerTypeFont(family);
                break;
            }
        }
    }
}

QString familyForFontName(const QString& name)
{
    // A family this session registered is its own answer, even a generic
    // alias such as "monospace" that the database does not list.
    if (name.isEmpty() || familiesTried().contains(name) || QFontDatabase::hasFamily(name)) {
        return name;
    }
    const QString stem = name.section(QLatin1Char('-'), 0, 0).toLower();
    for (const QString& family : QFontDatabase::families()) {
        if (QString(family).remove(QLatin1Char(' ')).toLower() == stem) {
            return family;
        }
    }
    return name;
}

bool fontIsInstalled(const QString& name)
{
    // ponytail: cached for the process; a font installed while Pictura runs
    // stays missing until a restart.
    static QHash<QString, bool> cache;
    if (name.isEmpty()) {
        return true;
    }
    const auto hit = cache.constFind(name);
    if (hit != cache.constEnd()) {
        return *hit;
    }
    const auto spaceless = [](QString family) {
        return family.remove(QLatin1Char(' ')).toLower();
    };
    // The stem as written and without a vendor suffix: a family may itself end
    // in "ps" or "mt".
    QStringList stems{spaceless(name.section(QLatin1Char('-'), 0, 0))};
    for (const char* suffix : {"psmt", "mt", "ps"}) {
        if (stems.first().size() > 3 && stems.first().endsWith(QLatin1String(suffix))) {
            stems << stems.first().chopped(int(qstrlen(suffix)));
            break;
        }
    }
    bool installed = stems.contains(QStringLiteral("liberationsans"))
        || QFontDatabase::hasFamily(name);
    for (const QString& family : installed ? QStringList() : QFontDatabase::families()) {
        if (stems.contains(spaceless(family))) {
            installed = true;
            break;
        }
    }
    cache.insert(name, installed);
    return installed;
}

QStringList missingFonts(const QStringList& names)
{
    QStringList out;
    for (const QString& name : names) {
        if (!fontIsInstalled(name) && !out.contains(name)) {
            out << name;
        }
    }
    return out;
}

} // namespace pictura
