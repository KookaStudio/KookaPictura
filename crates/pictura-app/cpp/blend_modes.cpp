#include "blend_modes.h"

#include "api/engine_bridge.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QComboBox>

namespace pictura {

QString BlendEntry::displayName() const
{
    return QCoreApplication::translate("PicturaBlendModes", name.toUtf8().constData());
}

const QVector<BlendEntry>& blendModes()
{
    static const QVector<BlendEntry> entries = [] {
        QVector<BlendEntry> out;
        for (const BlendModeRow& row : blend_mode_rows()) {
            out.append({row.key, row.name, row.has_neutral});
        }
        return out;
    }();
    return entries;
}

int blendModeIndex(const QString& key)
{
    for (int i = 0; i < blendModes().size(); ++i) {
        if (blendModes().at(i).key == key) {
            return i;
        }
    }
    return -1;
}

QString blendModeName(const QString& key)
{
    const int index = blendModeIndex(key);
    return index >= 0 ? blendModes().at(index).displayName() : key;
}

QVector<BlendEntry> allowedBlendModes(const PictureView* view)
{
    if (!view) {
        return blendModes();
    }
    const QStringList allowed = allowed_blend_mode_keys(*view);
    QVector<BlendEntry> out;
    out.reserve(allowed.size());
    for (const BlendEntry& entry : blendModes()) {
        if (allowed.contains(entry.key)) {
            out.append(entry);
        }
    }
    return out;
}

int syncBlendCombo(QComboBox* combo, const PictureView* view, const QString& currentKey)
{
    QVector<BlendEntry> desired = allowedBlendModes(view);
    const int currentEngineIndex = blendModeIndex(currentKey);
    bool currentAllowed = false;
    for (const BlendEntry& entry : desired) {
        if (entry.key == currentKey) {
            currentAllowed = true;
            break;
        }
    }
    const bool appendCurrent = currentEngineIndex >= 0 && !currentAllowed;
    if (appendCurrent) {
        desired.append(blendModes().at(currentEngineIndex));
    }
    bool same = combo->count() == desired.size();
    for (int i = 0; same && i < desired.size(); ++i) {
        same = combo->itemData(i).toString() == desired.at(i).key
               && combo->itemText(i) == desired.at(i).displayName();
    }
    // An appended conflict entry must be disabled, so an enabled one is stale.
    if (same && appendCurrent) {
        if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
            if (auto* item = model->item(combo->count() - 1)) {
                same = !item->isEnabled();
            }
        }
    }
    if (!same) {
        const QSignalBlocker block(combo);
        combo->clear();
        for (const BlendEntry& entry : desired) {
            combo->addItem(entry.displayName(), entry.key);
        }
        if (appendCurrent) {
            if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
                if (auto* item = model->item(combo->count() - 1)) {
                    item->setEnabled(false);
                }
            }
        }
    }
    return combo->findData(currentKey);
}

} // namespace pictura
