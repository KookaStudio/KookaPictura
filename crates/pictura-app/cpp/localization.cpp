#include "localization.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QHash>
#include <QtCore/QResource>
#include <QtCore/QSaveFile>
#include <QtCore/QStandardPaths>
#include <QtCore/QUuid>
#include <QtCore/QXmlStreamReader>
#include <QtGui/QTextDocument>

#include <optional>
#include <utility>

static void initTranslationResources()
{
    static const bool initialized = [] {
        Q_INIT_RESOURCE(pictura_translations);
        return true;
    }();
    Q_UNUSED(initialized);
}

namespace pictura {
namespace {

constexpr qsizetype kMaxCatalogBytes = 8 * 1024 * 1024;
constexpr qsizetype kMaxCatalogEntries = 20000;
constexpr qsizetype kMaxMessageChars = 8192;
constexpr int kMaxXmlDepth = 16;
constexpr char kImportPrefix[] = "import:";

struct MessageKey {
    QString context;
    QString source;
    QString disambiguation;

    bool operator==(const MessageKey& other) const
    {
        return context == other.context && source == other.source
            && disambiguation == other.disambiguation;
    }
};

size_t qHash(const MessageKey& key, size_t seed = 0) noexcept
{
    return qHashMulti(seed, key.context, key.source, key.disambiguation);
}

struct Catalog {
    QLocale language;
    QHash<MessageKey, QString> messages;
};

bool hasPlainTextRendering(const QString& context)
{
    return context == QLatin1String("PicturaCommands")
        || context == QLatin1String("PicturaTools")
        || context == QLatin1String("PicturaFiles")
        || context == QLatin1String("pictura::NewDocumentDialog")
        || context == QLatin1String("pictura::PreferencesDialog")
        || context == QLatin1String("pictura::UpdatePreferencesPage")
        || context == QLatin1String("pictura::AgentControlPage");
}

QString readMessageText(QXmlStreamReader& xml)
{
    QString text;
    while (!xml.atEnd()) {
        const auto token = xml.readNext();
        if (token == QXmlStreamReader::EndElement) {
            break;
        }
        if (token == QXmlStreamReader::Characters) {
            text += xml.text();
        } else if (token == QXmlStreamReader::StartElement) {
            if (xml.name() != QLatin1String("byte")) {
                xml.raiseError(QStringLiteral("Unexpected element in translation text."));
                break;
            }
            auto encoded = xml.attributes().value(QStringLiteral("value"));
            const bool hex = encoded.startsWith(QLatin1Char('x'));
            if (hex) {
                encoded = encoded.sliced(1);
            }
            bool ok = false;
            const auto value = encoded.toUInt(&ok, hex ? 16 : 10);
            if (!ok || value == 0 || value > 0xff
                || xml.readNext() != QXmlStreamReader::EndElement) {
                xml.raiseError(QStringLiteral("Invalid byte in translation text."));
                break;
            }
            text += QChar(static_cast<ushort>(value));
        }
        if (text.size() > kMaxMessageChars) {
            xml.raiseError(QStringLiteral("Translation text is too long."));
            break;
        }
    }
    return text;
}

void skipMetadata(QXmlStreamReader& xml, int depth)
{
    while (xml.readNextStartElement()) {
        if (depth >= kMaxXmlDepth) {
            xml.raiseError(QStringLiteral("Translation XML is too deeply nested."));
            return;
        }
        skipMetadata(xml, depth + 1);
    }
}

void readMessage(QXmlStreamReader& xml, const QString& context, Catalog& catalog,
                 const QTranslator& bundledMessages)
{
    MessageKey key{context, {}, {}};
    QString translation;
    bool finished = false;
    const bool supported = !xml.attributes().hasAttribute(QStringLiteral("id"))
        && xml.attributes().value(QStringLiteral("numerus")) != QLatin1String("yes");
    if (!supported) {
        skipMetadata(xml, 3);
        return;
    }
    while (xml.readNextStartElement()) {
        if (xml.name() == QLatin1String("source")) {
            key.source = readMessageText(xml);
        } else if (xml.name() == QLatin1String("comment")) {
            key.disambiguation = readMessageText(xml);
        } else if (xml.name() == QLatin1String("translation")) {
            finished = xml.attributes().value(QStringLiteral("type")).isEmpty();
            if (finished && !xml.attributes().hasAttribute(QStringLiteral("variants"))) {
                translation = readMessageText(xml);
            } else {
                skipMetadata(xml, 4);
            }
        } else {
            skipMetadata(xml, 4);
        }
    }
    if (xml.hasError() || !finished || translation.isEmpty() || key.source.isEmpty()) {
        return;
    }
    if (bundledMessages.translate(key.context.toUtf8().constData(), key.source.toUtf8().constData(),
                                  key.disambiguation.toUtf8().constData()).isNull()) {
        return;
    }
    if (!hasPlainTextRendering(key.context)
        && (translation.contains(QLatin1Char('<')) || Qt::mightBeRichText(key.source))) {
        return;
    }
    const auto existing = catalog.messages.constFind(key);
    if (existing != catalog.messages.cend()) {
        if (*existing != translation) {
            xml.raiseError(QStringLiteral("Conflicting translations for the same message."));
        }
        return;
    }
    if (catalog.messages.size() >= kMaxCatalogEntries) {
        xml.raiseError(QStringLiteral("Too many translation messages."));
        return;
    }
    catalog.messages.insert(std::move(key), std::move(translation));
}

void readContext(QXmlStreamReader& xml, Catalog& catalog, const QTranslator& bundledMessages)
{
    QString context;
    while (xml.readNextStartElement()) {
        if (xml.name() == QLatin1String("name")) {
            context = readMessageText(xml);
        } else if (xml.name() == QLatin1String("message") && !context.isEmpty()) {
            readMessage(xml, context, catalog, bundledMessages);
        } else {
            skipMetadata(xml, 3);
        }
    }
}

std::optional<Catalog> parseCatalog(const QByteArray& bytes)
{
    QXmlStreamReader xml(bytes);
    while (!xml.atEnd() && !xml.isStartElement()) {
        xml.readNext();
        if (xml.isDTD()
            && (xml.dtdName() != QLatin1String("TS") || !xml.dtdPublicId().isEmpty()
                || !xml.dtdSystemId().isEmpty() || !xml.entityDeclarations().isEmpty()
                || !xml.notationDeclarations().isEmpty() || xml.text().contains(QLatin1Char('[')))) {
            return std::nullopt;
        }
    }
    if (xml.hasError() || xml.name() != QLatin1String("TS") || !xml.namespaceUri().isEmpty()) {
        return std::nullopt;
    }
    const auto version = xml.attributes().value(QStringLiteral("version"));
    if (!version.isEmpty() && version != QLatin1String("2.1") && version != QLatin1String("2.0")) {
        return std::nullopt;
    }
    Catalog catalog{QLocale(xml.attributes().value(QStringLiteral("language")).toString()), {}};
    if (catalog.language.language() == QLocale::C) {
        return std::nullopt;
    }
    initTranslationResources();
    QTranslator bundledMessages;
    if (!bundledMessages.load(QStringLiteral(":/i18n/pictura_de.qm"))) {
        return std::nullopt;
    }
    while (xml.readNextStartElement()) {
        if (xml.name() == QLatin1String("dependencies")) {
            return std::nullopt;
        }
        if (xml.name() == QLatin1String("context")) {
            readContext(xml, catalog, bundledMessages);
        } else {
            skipMetadata(xml, 2);
        }
    }
    while (!xml.atEnd()) {
        xml.readNext();
    }
    if (xml.hasError() || catalog.messages.isEmpty()) {
        return std::nullopt;
    }
    return catalog;
}

QByteArray readCatalogFile(const QString& path)
{
    QFile file(path);
    if (!QFileInfo(path).isFile() || !file.open(QIODevice::ReadOnly)) {
        return {};
    }
    QByteArray bytes = file.read(kMaxCatalogBytes + 1);
    return bytes.size() <= kMaxCatalogBytes && file.error() == QFile::NoError ? bytes : QByteArray{};
}

class CommunityTranslator final : public QTranslator {
public:
    explicit CommunityTranslator(Catalog catalog) : messages_(std::move(catalog.messages)) {}

    bool isEmpty() const override { return messages_.isEmpty(); }

    QString translate(const char* context, const char* sourceText, const char* disambiguation,
                      int n) const override
    {
        if (n >= 0 || !context || !sourceText) {
            return {};
        }
        MessageKey key{QString::fromUtf8(context), QString::fromUtf8(sourceText),
                       QString::fromUtf8(disambiguation)};
        const auto match = messages_.constFind(key);
        if (match != messages_.cend()) {
            return *match;
        }
        if (!key.disambiguation.isEmpty()) {
            key.disambiguation.clear();
            return messages_.value(key);
        }
        return {};
    }

private:
    QHash<MessageKey, QString> messages_;
};

QString importPath(const QString& selection)
{
    if (!selection.startsWith(QLatin1String(kImportPrefix))) {
        return {};
    }
    const QString id = selection.sliced(qsizetype(sizeof(kImportPrefix) - 1));
    const QUuid uuid = QUuid::fromString(id);
    if (uuid.isNull() || uuid.toString(QUuid::WithoutBraces) != id) {
        return {};
    }
    return QDir(uiTranslationsDirectory()).filePath(id + QStringLiteral(".ts"));
}

QString systemLanguage(const QStringList& preferred)
{
    for (const auto& tag : preferred) {
        const auto language = QLocale(tag).language();
        switch (language) {
        case QLocale::English: return QStringLiteral("en");
        case QLocale::German: return QStringLiteral("de");
        case QLocale::Finnish: return QStringLiteral("fi");
        default: break;
        }
    }
    return QStringLiteral("en");
}

} // namespace

QString uiTranslationsDirectory()
{
    const auto xdg = qEnvironmentVariable("XDG_DATA_HOME");
    const auto base = xdg.isEmpty()
        ? QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation)
        : xdg;
    return QDir(base).filePath(QStringLiteral("kooka-pictura/translations"));
}

QList<UiLanguageOption> availableUiLanguages()
{
    QList<UiLanguageOption> options{
        {QStringLiteral("system"), QCoreApplication::translate("PicturaFiles", "System")},
        {QStringLiteral("en"), QStringLiteral("English")},
        {QStringLiteral("de"), QStringLiteral("Deutsch")},
        {QStringLiteral("fi"), QStringLiteral("Suomi")},
    };
    QHash<QString, int> labelCounts;
    const auto files = QDir(uiTranslationsDirectory()).entryInfoList(
        {QStringLiteral("*.ts")}, QDir::Files, QDir::Time | QDir::Reversed);
    for (const auto& file : files) {
        const QString id = QLatin1String(kImportPrefix) + file.completeBaseName();
        if (const auto path = importPath(id); !path.isEmpty()) {
            if (const auto catalog = parseCatalog(readCatalogFile(path))) {
                const auto native = catalog->language.nativeLanguageName();
                const auto base = native.isEmpty() ? catalog->language.name() : native;
                const int count = ++labelCounts[base];
                const auto label = count == 1
                    ? QCoreApplication::translate("PicturaFiles", "%1 (Community)").arg(base)
                    : QCoreApplication::translate("PicturaFiles", "%1 (Community %2)").arg(base).arg(count);
                options.push_back({id, label});
            }
        }
    }
    return options;
}

QString importUiTranslation(const QString& sourcePath, QString* error)
{
    if (error) {
        error->clear();
    }
    const auto bytes = sourcePath.endsWith(QLatin1String(".ts"), Qt::CaseInsensitive)
        ? readCatalogFile(sourcePath) : QByteArray{};
    if (!parseCatalog(bytes)) {
        if (error) {
            *error = QCoreApplication::translate(
                "PicturaFiles", "The file is not a compatible Qt Linguist translation.");
        }
        return {};
    }
    const auto id = QStringLiteral("import:") + QUuid::createUuid().toString(QUuid::WithoutBraces);
    QSaveFile output(importPath(id));
    if (!QDir().mkpath(uiTranslationsDirectory()) || !output.open(QIODevice::WriteOnly)
        || output.write(bytes) != bytes.size() || !output.commit()) {
        if (error) {
            *error = QCoreApplication::translate("PicturaFiles", "The translation could not be saved.");
        }
        return {};
    }
    return id;
}

std::unique_ptr<QTranslator> installUiTranslation(
    const QString& selection, const QStringList& systemLanguages)
{
    if (const auto path = importPath(selection); !path.isEmpty()) {
        if (auto catalog = parseCatalog(readCatalogFile(path))) {
            auto translator = std::make_unique<CommunityTranslator>(std::move(*catalog));
            if (QCoreApplication::installTranslator(translator.get())) {
                return translator;
            }
        }
    }
    const QString system = systemLanguage(systemLanguages);
    const bool bundled = selection == QLatin1String("en") || selection == QLatin1String("de")
        || selection == QLatin1String("fi");
    const QString selected = bundled ? selection : system;
    if (selected != QLatin1String("en")) {
        initTranslationResources();
        auto translator = std::make_unique<QTranslator>();
        if (translator->load(QStringLiteral(":/i18n/pictura_%1.qm").arg(selected))
            && QCoreApplication::installTranslator(translator.get())) {
            return translator;
        }
        if (selected != system && system != QLatin1String("en")
            && translator->load(QStringLiteral(":/i18n/pictura_%1.qm").arg(system))
            && QCoreApplication::installTranslator(translator.get())) {
            return translator;
        }
    }
    return {};
}

QLocale uiNumericLocale()
{
    QLocale locale;
    locale.setNumberOptions(locale.numberOptions() | QLocale::OmitGroupSeparator
                            | QLocale::RejectGroupSeparator);
    return locale;
}

QString plainUiTooltip(const QString& text)
{
    return Qt::mightBeRichText(text) ? QStringLiteral("<qt>%1</qt>").arg(text.toHtmlEscaped())
                                    : text;
}

} // namespace pictura
