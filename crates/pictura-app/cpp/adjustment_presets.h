#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QComboBox>

#include <functional>

namespace pictura {

// A built-in look: `build` applies it to the current block (a preset that only
// changes one parameter) and `matches` says whether a block already is it.
struct BuiltinPreset {
    QString name;
    std::function<QByteArray(const QByteArray&)> build;
    std::function<bool(const QByteArray&)> matches;
};

// The established built-ins for `kind`; empty for a kind with no sourced list.
const QList<BuiltinPreset>& builtinPresets(const QString& kind);

// The engine's saved presets for `kind`, in store order.
QStringList savedPresetNames(const QString& kind);
QByteArray savedPresetBlock(const QString& kind, const QString& name);
bool saveAdjustmentPreset(const QString& kind, const QString& name, const QByteArray& block);
bool deleteAdjustmentPreset(const QString& kind, const QString& name);
bool adjustmentPresetCanSave(const QString& kind);
bool adjustmentPresetSupportsMenu(const QString& kind);
QString adjustmentPresetKind(const QByteArray& block);
bool adjustmentPresetMatches(const QByteArray& a, const QByteArray& b);

// `kind`'s dialog default block.
QByteArray defaultPresetBlock(const QString& kind);

// The shared Preset menu: Default, the kind's built-ins, the saved presets, and
// Custom. `presetChosen` carries the block the owner should apply.
class PresetCombo : public QComboBox {
    Q_OBJECT

public:
    explicit PresetCombo(QWidget* parent = nullptr);

    // Fill for `kind` (when it changed) and mark the entry matching `block`.
    void setBlock(const QString& kind, const QByteArray& block);
    // Re-read the saved presets from disk.
    void reload();
    QString kind() const { return kind_; }

signals:
    void presetChosen(const QByteArray& block);

private:
    struct Entry {
        QString name;
        QByteArray block;
        std::function<QByteArray(const QByteArray&)> build;
        std::function<bool(const QByteArray&)> matches;
        bool selectable = false;
    };

    void rebuild();
    void updateCurrent();
    void activate(int index);

    QString kind_;
    QByteArray block_;
    QList<Entry> entries_;
};

} // namespace pictura
