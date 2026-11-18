#include "batch_options.h"

#include "dialogs.h"

#include "api/engine_bridge.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QRegularExpression>

namespace pictura {
namespace {

QString compactJson(const QJsonObject& object)
{
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QString sourceKey(BatchOptions::Source source)
{
    return source == BatchOptions::Source::OpenedFiles ? QStringLiteral("openedFiles")
                                                       : QStringLiteral("folder");
}

QString destinationKey(BatchOptions::Destination destination)
{
    switch (destination) {
    case BatchOptions::Destination::SaveAndClose:
        return QStringLiteral("saveAndClose");
    case BatchOptions::Destination::Folder:
        return QStringLiteral("folder");
    default:
        return QStringLiteral("none");
    }
}

} // namespace

QJsonObject BatchOptions::namingJson() const
{
    return QJsonObject{
        {QStringLiteral("fields"), QJsonArray::fromStringList(namingFields)},
        {QStringLiteral("startingSerial"), qMax(1, startingSerial)},
        {QStringLiteral("windows"), windowsCompatible},
        {QStringLiteral("mac"), macCompatible},
    };
}

QJsonObject BatchOptions::toJson() const
{
    return QJsonObject{
        {QStringLiteral("set"), set},
        {QStringLiteral("action"), action},
        {QStringLiteral("source"), sourceKey(source)},
        {QStringLiteral("sourceFolder"), sourceFolder},
        {QStringLiteral("includeSubfolders"), includeSubfolders},
        {QStringLiteral("overrideOpen"), overrideOpen},
        {QStringLiteral("suppressOpen"), suppressOpen},
        {QStringLiteral("suppressProfile"), suppressProfile},
        {QStringLiteral("destination"), destinationKey(destination)},
        {QStringLiteral("destinationFolder"), destinationFolder},
        {QStringLiteral("overrideSave"), overrideSave},
        {QStringLiteral("naming"), namingJson()},
        {QStringLiteral("errors"),
         errors == Errors::Log ? QStringLiteral("log") : QStringLiteral("stop")},
        {QStringLiteral("errorLog"), errorLog},
    };
}

BatchOptions BatchOptions::fromJson(const QJsonObject& json)
{
    BatchOptions options;
    options.set = json.value(QStringLiteral("set")).toString();
    options.action = json.value(QStringLiteral("action")).toString();
    options.source = json.value(QStringLiteral("source")).toString() == QLatin1String("openedFiles")
        ? Source::OpenedFiles
        : Source::Folder;
    options.sourceFolder = json.value(QStringLiteral("sourceFolder")).toString();
    options.includeSubfolders = json.value(QStringLiteral("includeSubfolders")).toBool();
    options.overrideOpen = json.value(QStringLiteral("overrideOpen")).toBool();
    options.suppressOpen = json.value(QStringLiteral("suppressOpen")).toBool();
    options.suppressProfile = json.value(QStringLiteral("suppressProfile")).toBool();
    const QString destination = json.value(QStringLiteral("destination")).toString();
    options.destination = destination == QLatin1String("saveAndClose")
        ? Destination::SaveAndClose
        : destination == QLatin1String("folder") ? Destination::Folder : Destination::None;
    options.destinationFolder = json.value(QStringLiteral("destinationFolder")).toString();
    options.overrideSave = json.value(QStringLiteral("overrideSave")).toBool();
    const QJsonObject naming = json.value(QStringLiteral("naming")).toObject();
    if (naming.contains(QStringLiteral("fields"))) {
        options.namingFields.clear();
        for (const QJsonValue& field : naming.value(QStringLiteral("fields")).toArray()) {
            options.namingFields << field.toString();
        }
    }
    options.startingSerial = qMax(1, naming.value(QStringLiteral("startingSerial")).toInt(1));
    options.windowsCompatible = naming.value(QStringLiteral("windows")).toBool();
    options.macCompatible = naming.value(QStringLiteral("mac")).toBool();
    options.errors = json.value(QStringLiteral("errors")).toString() == QLatin1String("log")
        ? Errors::Log
        : Errors::Stop;
    options.errorLog = json.value(QStringLiteral("errorLog")).toString();
    return options;
}

QStringList batchTokenLabels()
{
    return batch_token_labels();
}

QString batchRenderName(const QJsonObject& naming, const QString& inputName, const QString& ext,
                        int position, const QDate& date)
{
    return batch_render_name(compactJson(naming), inputName, ext, position, date.year(),
                             date.month(), date.day());
}

bool batchNamingHasUniqueField(const QJsonObject& naming)
{
    return batch_naming_has_unique_field(compactJson(naming));
}

QStringList batchOpenableExtensions()
{
    // The first Open filter is All Formats; its patterns are the openable set.
    static const QStringList extensions = [] {
        QStringList out;
        const QString all = openFileFilters().value(0);
        static const QRegularExpression pattern(QStringLiteral("\\*\\.(\\w+)"));
        for (auto it = pattern.globalMatch(all); it.hasNext();) {
            out << it.next().captured(1).toLower();
        }
        return out;
    }();
    return extensions;
}

QStringList batchResolveFolder(const QString& root, bool recurse)
{
    return batch_resolve_folder(root, recurse, batchOpenableExtensions());
}

QString batchOverrideSavePath(const BatchOptions& options, const QString& input,
                              const QString& format, int position, const QDate& date)
{
    return batch_override_save_path(compactJson(options.toJson()), input, format, position,
                                    date.year(), date.month(), date.day());
}

QString batchErrorLogLine(const QString& input, const QString& message)
{
    return batch_error_log_line(input, message);
}

} // namespace pictura
