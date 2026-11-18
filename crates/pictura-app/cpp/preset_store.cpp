#include "preset_store.h"

#include "panels/swatches_panel.h"
#include "panels/brush_preset_picker.h"
#include "layer_style_contour.h"
#include "session.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLockFile>
#include <QtCore/QPointer>
#include <QtCore/QRegularExpression>
#include <QtCore/QSaveFile>
#include <QtCore/QSet>
#include <QtCore/QUuid>
#include <QtCore/QtEndian>

#include <algorithm>

namespace pictura {

struct StagedPresetImport {
    QString path;
    explicit StagedPresetImport(QString name) : path(std::move(name)) {}
    ~StagedPresetImport() { QFile::remove(path); }
};

namespace {

constexpr qsizetype maxPresetBytes = 128 * 1024 * 1024;
constexpr std::array<const char*, presetTypeCount> keys = {
    "Brushes", "Swatches", "Gradients", "Styles", "Patterns", "Contours", "Custom Shapes", "Tools"
};
constexpr std::array<const char*, presetTypeCount> extensions = {
    "*.abr", "*.aco *.ase", "*.grd", "*.asl", "*.pat", "*.shc", "*.csh", "*.tpl"
};

QString currentListDir()
{
    return QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/preset-current");
}

bool fail(QString* error, const QString& message)
{
    if (error) {
        *error = message;
    }
    return false;
}

::rust::Slice<const std::uint8_t> slice(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), size_t(bytes.size())};
}

QByteArray asBytes(const ::rust::Vec<std::uint8_t>& bytes)
{
    return QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
}

bool writeFile(const QString& path, const QByteArray& bytes, QString* error)
{
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit()) {
        return fail(error, file.errorString());
    }
    return true;
}

bool readFile(const QString& path, QByteArray* bytes, QString* error, qsizetype max = maxPresetBytes)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return fail(error, file.errorString());
    }
    if (file.size() > max) {
        return fail(error, QObject::tr("Preset file %1 exceeds its size limit.").arg(path));
    }
    *bytes = file.read(max + 1);
    if (bytes->size() > max || !file.atEnd() || file.error() != QFileDevice::NoError) {
        return fail(error, QObject::tr("Cannot read preset file %1 within its size limit.").arg(path));
    }
    return true;
}

bool readManifest(QJsonObject* manifest, QString* error)
{
    const QString path = currentListDir() + QStringLiteral("/active.json");
    if (!QFileInfo::exists(path)) {
        *manifest = {};
        return true;
    }
    QByteArray bytes;
    if (!readFile(path, &bytes, error, 4096)) {
        return false;
    }
    const auto doc = QJsonDocument::fromJson(bytes);
    if (!doc.isObject()) {
        return fail(error, QObject::tr("The active preset manifest is invalid."));
    }
    *manifest = doc.object();
    return true;
}

bool readStored(const QJsonObject& manifest, PresetType type, QByteArray* bytes, QString* error,
                qsizetype max = maxPresetBytes)
{
    const auto key = presetTypeKey(type);
    if (!manifest.contains(key)) {
        return true;
    }
    const auto name = manifest.value(key).toString();
    if (!name.endsWith(QStringLiteral(".kpset")) || QFileInfo(name).fileName() != name
        || name.contains(QLatin1Char('\\')) || name.contains(QLatin1Char('/'))) {
        return fail(error, QObject::tr("Invalid current preset path for %1.").arg(key));
    }
    return readFile(currentListDir() + QLatin1Char('/') + name, bytes, error, max);
}

bool encode(PresetType type, const QString& extension, const PresetRows& rows, QByteArray* bytes,
            QString* error)
{
    QByteArray input;
    for (const auto& row : rows) {
        const auto name = row.name.toUtf8();
        if (input.size() + name.size() + row.native.size() + 8 > maxPresetBytes || row.native.isEmpty()) {
            return fail(error, QObject::tr("Cannot encode preset %1.").arg(row.name));
        }
        for (const auto* field : {&name, &row.native}) {
            const auto length = qToBigEndian(quint32(field->size()));
            input.append(reinterpret_cast<const char*>(&length), sizeof(length));
            input.append(*field);
        }
    }
    ::rust::Vec<std::uint8_t> output;
    const auto failure = preset_encode_set(int(type), extension, slice(input), output);
    if (!failure.isEmpty()) {
        return fail(error, failure);
    }
    *bytes = asBytes(output);
    return true;
}

PresetRows defaults(PresetType type)
{
    PresetRows rows;
    if (type == PresetType::Swatches) {
        for (const auto& swatch : SwatchGrid::defaultSwatches()) {
            rows.push_back(makeSwatchPreset(swatch.name, swatch.color));
        }
        return rows;
    }
    QStringList names;
    switch (type) {
    case PresetType::Brushes: names = BrushPresetPicker::builtInNames(); break;
    case PresetType::Contours: names = builtInContourNames(); break;
    case PresetType::Gradients:
        for (int i = 0; i < gradient_preset_count(); ++i) { names.push_back(gradient_preset_name(i)); }
        break;
    case PresetType::Patterns: {
        const int stampCount = stamp_pattern_count();
        const int count = managed_pattern_count();
        for (int row = 0; row < count; ++row) {
            const int index = (row + stampCount) % count;
            rows.push_back(makeBuiltinPreset(type, stamp_pattern_name(index), index));
        }
        return rows;
    }
    case PresetType::CustomShapes:
        for (int i = 0; i < shape_custom_count(); ++i) { names.push_back(shape_custom_name(i)); }
        break;
    default: break;
    }
    for (int index = 0; index < names.size(); ++index) {
        rows.push_back(makeBuiltinPreset(type, names.at(index), index));
    }
    return rows;
}

PresetRows initialRows(PresetType type, const SessionState& state)
{
    if (type != PresetType::Swatches || !state.swatchesLoaded) {
        return defaults(type);
    }
    PresetRows rows;
    for (const auto& value : state.swatches) {
        const auto item = value.toObject();
        const QColor color(item.value(QStringLiteral("color")).toString());
        if (color.isValid()) {
            rows.push_back(makeSwatchPreset(item.value(QStringLiteral("name")).toString(), color));
        }
    }
    return rows.isEmpty() && !state.swatches.isEmpty() ? defaults(type) : rows;
}

bool publish(const PresetSnapshot& snapshot, QString* error, const std::shared_ptr<StagedPresetImport>& staged = {})
{
    if (snapshot.isEmpty()) {
        return true;
    }
    const auto folder = currentListDir();
    if (!QDir().mkpath(folder)) {
        return fail(error, QObject::tr("Cannot create the current preset directory."));
    }
    QLockFile lock(folder + QStringLiteral("/active.lock"));
    if (!lock.tryLock()) {
        return fail(error, QObject::tr("Another instance is writing current presets."));
    }
    QJsonObject manifest;
    if (!readManifest(&manifest, error)) {
        return false;
    }
    QStringList used;
    const auto generation = QUuid::createUuid().toString(QUuid::WithoutBraces);
    for (auto it = snapshot.cbegin(); it != snapshot.cend(); ++it) {
        const auto key = presetTypeKey(it.key());
        const auto name = generation + QLatin1Char('-') + key + QStringLiteral(".kpset");
        const auto path = folder + QLatin1Char('/') + name;
        const bool written = staged ? QFile::rename(staged->path, path) : writeFile(path, it.value(), error);
        if (!written) {
            if (staged) fail(error, QObject::tr("Cannot publish staged preset list."));
            for (const auto& file : used) {
                QFile::remove(file);
            }
            return false;
        }
        used.push_back(path);
        manifest.insert(key, name);
    }
    QLockFile readers(folder + QStringLiteral("/read.lock"));
    if (!readers.tryLock()) {
        for (const auto& file : used) QFile::remove(file);
        return fail(error, QObject::tr("Cannot lock current preset lists."));
    }
    if (!writeFile(folder + QStringLiteral("/active.json"), QJsonDocument(manifest).toJson(), error)) {
        for (const auto& file : used) {
            QFile::remove(file);
        }
        return false;
    }
    QSet<QString> active;
    for (auto it = manifest.constBegin(); it != manifest.constEnd(); ++it) active.insert(it.value().toString());
    const QRegularExpression generationPattern(QStringLiteral("^[0-9a-fA-F-]{36}-.+\\.kpset$"));
    for (const auto& name : QDir(folder).entryList({QStringLiteral("*.kpset")}, QDir::Files)) {
        if (generationPattern.match(name).hasMatch() && !active.contains(name)) QFile::remove(folder + QLatin1Char('/') + name);
    }
    return true;
}

} // namespace

QString presetTypeName(PresetType type) { return QCoreApplication::translate("PresetManager", keys.at(int(type))); }

QString presetTypeKey(PresetType type)
{
    return QString::fromLatin1(keys.at(int(type))).replace(QLatin1Char(' '), QLatin1Char('-')).toLower();
}

PresetRow makeSwatchPreset(const QString& name, const QColor& color)
{
    return {0, name, color, asBytes(preset_make_swatch(name, color.rgba())), {}};
}

PresetRow makeBuiltinPreset(PresetType type, const QString& name, int index)
{
    return {0, name, {}, asBytes(preset_make_builtin(int(type), name, index)), {{QStringLiteral("index"), index}}};
}

QString presetFileFilter(PresetType type, bool save)
{
    QString filters = QObject::tr("Pictura Preset Sets (*.kpset)");
    if (type == PresetType::Swatches && save) {
        filters += QObject::tr(";;Adobe Color Swatches (*.aco);;Adobe Swatch Exchange (*.ase)");
    } else if (!save) {
        filters += QObject::tr(";;Photoshop preset libraries (%1)").arg(QString::fromLatin1(extensions.at(int(type))));
    }
    return filters;
}

QString presetLibraryFolder(PresetType type)
{
    const auto home = qEnvironmentVariable("XDG_DATA_HOME", QDir::homePath() + QStringLiteral("/.local/share"));
    return home + QStringLiteral("/kooka-pictura/presets/") + presetTypeKey(type);
}

QStringList discoveredPresetFiles(PresetType type)
{
    QStringList roots{presetLibraryFolder(type)};
    const auto dirs = qEnvironmentVariable("XDG_DATA_DIRS", QStringLiteral("/usr/local/share:/usr/share:/app/share"));
    for (const auto& directory : dirs.split(QLatin1Char(':'), Qt::SkipEmptyParts)) {
        roots.push_back(directory + QStringLiteral("/kooka-pictura/presets/") + presetTypeKey(type));
    }
    QStringList files;
    QSet<QString> seen;
    auto filters = QString::fromLatin1(extensions.at(int(type))).split(QLatin1Char(' '));
    filters.push_back(QStringLiteral("*.kpset"));
    for (const auto& root : roots) {
        const QDir folder(root);
        for (const auto& name : folder.entryList(filters, QDir::Files | QDir::Readable)) {
            if (!seen.contains(name)) {
                seen.insert(name);
                files.push_back(folder.absoluteFilePath(name));
            }
        }
    }
    return files;
}

PresetImport readPresetLibrary(PresetType type, const QString& extension, const QByteArray& bytes)
{
    const auto read = preset_read_set(int(type), extension, slice(bytes));
    const auto error = preset_set_error(*read);
    if (!error.isEmpty()) {
        return {false, false, {}, error};
    }
    PresetImport result;
    result.ok = true;
    result.diagnostics = preset_set_warnings(*read);
    const int size = preset_set_size(*read);
    result.explicitEmpty = size == 0 && extension.compare(QStringLiteral("kpset"), Qt::CaseInsensitive) == 0;
    for (int index = 0; index < size; ++index) {
        auto native = asBytes(preset_row_bytes(*read, index));
        if (native.isEmpty()) {
            return {false, false, {}, QObject::tr("Cannot preserve preset record %1.").arg(index + 1)};
        }
        const auto color = type == PresetType::Swatches ? QColor::fromRgba(preset_row_rgba(*read, index)) : QColor();
        const auto values = QJsonDocument::fromJson(preset_row_values(*read, index).toUtf8()).object();
        result.rows.push_back({0, preset_row_name(*read, index), color, std::move(native), values,
                               preset_row_preview(*read, index)});
    }
    return result;
}

PresetImport readPresetFile(const QString& path, PresetType type)
{
    QByteArray bytes;
    QString error;
    if (!readFile(path, &bytes, &error)) {
        return {false, false, {}, error};
    }
    return readPresetLibrary(type, QFileInfo(path).suffix(), bytes);
}

bool savePresetFile(const QString& path, PresetType type, const PresetRows& rows, QString* error)
{
    QByteArray bytes;
    return encode(type, QFileInfo(path).suffix(), rows, &bytes, error) && writeFile(path, bytes, error);
}

bool collectPresetSnapshot(PresetSnapshot* snapshot, QString* error)
{
    if (!QDir().mkpath(currentListDir())) return fail(error, QObject::tr("Cannot create the current preset directory."));
    QLockFile readers(currentListDir() + QStringLiteral("/read.lock"));
    if (!readers.tryLock(10000)) return fail(error, QObject::tr("Cannot lock current preset lists."));
    QJsonObject manifest;
    if (!readManifest(&manifest, error)) {
        return false;
    }
    constexpr qsizetype maxSnapshotBytes = 192 * 1024 * 1024;
    PresetSnapshot storedBytes;
    qsizetype storedTotal = 0;
    for (int i = 0; i < presetTypeCount; ++i) {
        const auto type = PresetType(i);
        if (!readStored(manifest, type, &storedBytes[type], error,
                        std::min(maxPresetBytes, maxSnapshotBytes - storedTotal))) return false;
        if (storedBytes[type].size() > maxSnapshotBytes - storedTotal) {
            return fail(error, QObject::tr("Current preset lists exceed the settings backup size limit."));
        }
        storedTotal += storedBytes[type].size();
    }
    readers.unlock();
    const auto state = loadSession();
    qsizetype total = 0;
    for (int i = 0; i < presetTypeCount; ++i) {
        const auto type = PresetType(i);
        QByteArray bytes = std::move(storedBytes[type]);
        const bool stored = manifest.contains(presetTypeKey(type));
        if (!stored && !encode(type, QStringLiteral("kpset"), initialRows(type, state), &bytes, error)) {
            return false;
        }
        if (bytes.size() > maxSnapshotBytes - total) {
            return fail(error, QObject::tr("Current preset lists exceed the settings backup size limit."));
        }
        if (stored) {
            const auto result = preset_read_set(i, QStringLiteral("kpset"), slice(bytes));
            if (const auto failure = preset_set_error(*result); !failure.isEmpty()) {
                return fail(error, failure);
            }
        }
        total += bytes.size();
        snapshot->insert(type, std::move(bytes));
    }
    return true;
}

bool validatePresetSnapshot(const PresetSnapshot& snapshot, QString* error)
{
    for (auto it = snapshot.cbegin(); it != snapshot.cend(); ++it) {
        const auto result = preset_read_set(int(it.key()), QStringLiteral("kpset"), slice(it.value()));
        if (const auto failure = preset_set_error(*result); !failure.isEmpty()) {
            return fail(error, failure);
        }
    }
    return true;
}

bool publishPresetSnapshot(const PresetSnapshot& snapshot, QString* error)
{
    return validatePresetSnapshot(snapshot, error) && publish(snapshot, error);
}

PresetStore::PresetStore(QObject* parent) : QObject(parent)
{
    QDir().mkpath(currentListDir());
    QLockFile readers(currentListDir() + QStringLiteral("/read.lock"));
    QJsonObject manifest;
    QString error;
    bool loaded = readers.tryLock(10000);
    if (loaded) loaded = readManifest(&manifest, &error);
    else error = tr("Cannot lock current preset lists.");
    PresetSnapshot stored;
    std::array<QString, presetTypeCount> failures;
    if (loaded) {
        for (int i = 0; i < presetTypeCount; ++i) {
            const auto type = PresetType(i);
            readStored(manifest, type, &stored[type], &failures[i]);
        }
    }
    readers.unlock();
    const auto state = loadSession();
    for (int i = 0; i < presetTypeCount; ++i) {
        const auto type = PresetType(i);
        if (!loaded) {
            lists_[i] = defaults(type);
        } else if (!manifest.contains(presetTypeKey(type))) {
            lists_[i] = initialRows(type, state);
            if (type == PresetType::Swatches && state.swatchesLoaded) {
                replace(type, lists_[i], &error);
            }
        } else if (!failures[i].isEmpty()) {
            error = failures[i];
            lists_[i] = defaults(type);
        } else {
            auto report = readPresetLibrary(type, QStringLiteral("kpset"), stored.take(type));
            if (report.ok) lists_[i] = std::move(report.rows);
            else { error = report.diagnostics; lists_[i] = defaults(type); }
        }
        assignIds(lists_[i]);
        if (!error.isEmpty()) {
            qWarning().noquote() << error;
            error.clear();
        }
    }
}

PresetStore& PresetStore::current()
{
    static QMap<QString, QPointer<PresetStore>> stores;
    auto& store = stores[currentListDir()];
    if (!store) {
        store = new PresetStore(QCoreApplication::instance());
    }
    return *store;
}

const PresetRows& PresetStore::rows(PresetType type) const { return lists_.at(int(type)); }

bool PresetStore::replace(PresetType type, PresetRows rows, QString* error)
{
    QByteArray bytes;
    if (!encode(type, QStringLiteral("kpset"), rows, &bytes, error) || !publish({{type, bytes}}, error)) {
        return false;
    }
    assignIds(rows);
    lists_.at(int(type)) = std::move(rows);
    ++imports_.at(int(type));
    emit changed(int(type));
    return true;
}

bool PresetStore::append(PresetType type, const PresetRows& rows, QString* error)
{
    if (rows.isEmpty()) {
        return true;
    }
    auto updated = this->rows(type);
    for (auto row : rows) {
        row.id = 0;
        updated.push_back(std::move(row));
    }
    return replace(type, std::move(updated), error);
}

bool PresetStore::reset(PresetType type, bool appendDefaults, QString* error)
{
    return appendDefaults ? append(type, defaults(type), error) : replace(type, defaults(type), error);
}

bool PresetStore::resetAll(QString* error)
{
    std::array<PresetRows, presetTypeCount> rows;
    PresetSnapshot snapshot;
    for (int i = 0; i < presetTypeCount; ++i) {
        const auto type = PresetType(i);
        rows[i] = defaults(type);
        if (!encode(type, QStringLiteral("kpset"), rows[i], &snapshot[type], error)) {
            return false;
        }
    }
    if (!publish(snapshot, error)) {
        return false;
    }
    lists_ = std::move(rows);
    for (int i = 0; i < presetTypeCount; ++i) {
        assignIds(lists_[i]);
        ++imports_[i];
        emit changed(i);
    }
    return true;
}

std::shared_ptr<StagedPresetImport> stagePresetImport(PresetType type, const PresetRows& rows, QString* error)
{
    QByteArray bytes;
    if (!encode(type, QStringLiteral("kpset"), rows, &bytes, error)
        || !QDir().mkpath(currentListDir())) return {};
    auto staged = std::make_shared<StagedPresetImport>(currentListDir() + QStringLiteral("/staged-")
        + QUuid::createUuid().toString(QUuid::WithoutBraces));
    if (!writeFile(staged->path, bytes, error)) return {};
    return staged;
}

bool PresetStore::commitStagedImport(PresetType type, PresetRows rows,
                                     const std::shared_ptr<StagedPresetImport>& staged, QString* error)
{
    if (!staged || !publish({{type, {}}}, error, staged)) return false;
    assignIds(rows);
    lists_.at(int(type)) = std::move(rows);
    ++imports_.at(int(type));
    emit changed(int(type));
    return true;
}

bool PresetStore::commitImport(PresetType type, const PresetImport& report, bool replaceList,
                               bool acceptPartial, QString* error)
{
    if (!report.ok || (!report.explicitEmpty && report.rows.isEmpty())
        || (!acceptPartial && !report.diagnostics.isEmpty())) {
        return fail(error, report.diagnostics);
    }
    return replaceList ? replace(type, report.rows, error) : append(type, report.rows, error);
}

quint64 PresetStore::beginImport(PresetType type) { return ++imports_.at(int(type)); }
bool PresetStore::isCurrentImport(PresetType type, quint64 request) const { return request == imports_.at(int(type)); }
void PresetStore::cancelImport(PresetType type, quint64 request)
{
    if (isCurrentImport(type, request)) {
        ++imports_.at(int(type));
    }
}

void PresetStore::assignIds(PresetRows& rows)
{
    for (auto& row : rows) {
        if (!row.id) {
            row.id = nextId_++;
        }
    }
}

} // namespace pictura

#include "moc_preset_store.cpp"
