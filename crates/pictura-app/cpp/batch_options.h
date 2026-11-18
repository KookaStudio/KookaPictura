#pragma once

#include <QtCore/QDate>
#include <QtCore/QJsonObject>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

// File > Automate > Batch options: the C++ mirror of the batch engine's
// `BatchOptions` serde model (`pictura_script::batch`), crossing as JSON.
struct BatchOptions {
    enum class Source { Folder, OpenedFiles };
    enum class Destination { None, SaveAndClose, Folder };
    enum class Errors { Stop, Log };

    QString set;
    QString action;
    Source source = Source::Folder;
    QString sourceFolder;
    bool includeSubfolders = false;
    bool overrideOpen = false;
    bool suppressOpen = false;
    bool suppressProfile = false;
    Destination destination = Destination::None;
    QString destinationFolder;
    bool overrideSave = false;
    QStringList namingFields{QStringLiteral("Document Name"), QStringLiteral("extension")};
    int startingSerial = 1;
    bool windowsCompatible = false;
    bool macCompatible = false;
    Errors errors = Errors::Stop;
    QString errorLog;

    QJsonObject namingJson() const;
    QJsonObject toJson() const;
    static BatchOptions fromJson(const QJsonObject& json);
};

// The CS6 File Naming menu labels, in menu order.
QStringList batchTokenLabels();

// The output name for one input (`inputName` is a file name), saved as `ext`.
QString batchRenderName(const QJsonObject& naming, const QString& inputName, const QString& ext,
                        int position, const QDate& date);

// Whether the naming scheme has a document-name or serial field.
bool batchNamingHasUniqueField(const QJsonObject& naming);

// The suffixes a Folder source picks up: the Open dialog's All Formats list.
QStringList batchOpenableExtensions();

// The Folder source's inputs in batch order.
QStringList batchResolveFolder(const QString& root, bool recurse);

// Where an overridden Save As writes, or empty when not redirected.
QString batchOverrideSavePath(const BatchOptions& options, const QString& input,
                              const QString& format, int position, const QDate& date);

// One error-log line, newline-terminated.
QString batchErrorLogLine(const QString& input, const QString& message);

} // namespace pictura
