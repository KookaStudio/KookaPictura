#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QJsonObject>
#include <QtCore/QMap>
#include <QtCore/QObject>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QImage>

#include <array>
#include <memory>

namespace pictura {

enum class PresetType { Brushes, Swatches, Gradients, Styles, Patterns, Contours, CustomShapes, Tools };
inline constexpr int presetTypeCount = 8;

struct PresetRow {
    quint64 id = 0;
    QString name;
    QColor color;
    QByteArray native;
    QJsonObject values;
    QImage preview{};
};
using PresetRows = QList<PresetRow>;

struct PresetImport {
    bool ok = false;
    bool explicitEmpty = false;
    PresetRows rows;
    QString diagnostics;
};

PresetRow makeSwatchPreset(const QString& name, const QColor& color);
PresetRow makeBuiltinPreset(PresetType type, const QString& name, int index);
QString presetTypeName(PresetType type);
QString presetFileFilter(PresetType type, bool save);
QString presetLibraryFolder(PresetType type);
QStringList discoveredPresetFiles(PresetType type);
PresetImport readPresetLibrary(PresetType type, const QString& extension, const QByteArray& bytes);
PresetImport readPresetFile(const QString& path, PresetType type);
bool savePresetFile(const QString& path, PresetType type, const PresetRows& rows, QString* error = nullptr);

struct StagedPresetImport;
std::shared_ptr<StagedPresetImport> stagePresetImport(PresetType type, const PresetRows& rows, QString* error);

using PresetSnapshot = QMap<PresetType, QByteArray>;
bool collectPresetSnapshot(PresetSnapshot* snapshot, QString* error);
bool validatePresetSnapshot(const PresetSnapshot& snapshot, QString* error);
bool publishPresetSnapshot(const PresetSnapshot& snapshot, QString* error);
QString presetTypeKey(PresetType type);

class PresetStore : public QObject {
    Q_OBJECT

public:
    explicit PresetStore(QObject* parent = nullptr);
    static PresetStore& current();

    const PresetRows& rows(PresetType type) const;
    bool replace(PresetType type, PresetRows rows, QString* error = nullptr);
    bool append(PresetType type, const PresetRows& rows, QString* error = nullptr);
    bool reset(PresetType type, bool appendDefaults, QString* error = nullptr);
    bool resetAll(QString* error = nullptr);
    bool commitImport(PresetType type, const PresetImport& report, bool replaceList,
                      bool acceptPartial, QString* error = nullptr);
    bool commitStagedImport(PresetType type, PresetRows rows, const std::shared_ptr<StagedPresetImport>& staged, QString* error);
    quint64 beginImport(PresetType type);
    bool isCurrentImport(PresetType type, quint64 request) const;
    void cancelImport(PresetType type, quint64 request);

signals:
    void changed(int type);

private:
    void assignIds(PresetRows& rows);

    std::array<PresetRows, presetTypeCount> lists_;
    std::array<quint64, presetTypeCount> imports_{};
    quint64 nextId_ = 1;
};

} // namespace pictura
