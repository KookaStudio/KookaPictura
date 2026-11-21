#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>

class QComboBox;

namespace pictura {

class PictureView;

// One layer blend mode as the engine lists it (`BlendMode::LAYER_MODES`).
struct BlendEntry {
    QString key;
    QString name;
    bool hasNeutral = false;

    QString displayName() const;
};

// The 27 layer modes in engine order, read once from the engine. Shared by the
// Layers panel's blend combo and Mode filter, the New Layer dialog, the Layer
// Style dialog, and the Properties panel. The Layers panel, Mode filter and New
// Layer combos carry the engine key as item data; the Layer Style combo carries
// the engine index.
const QVector<BlendEntry>& blendModes();

// The layer modes the document behind `view` offers, in engine order; all 27
// when `view` is null or has no document. The engine owns the availability
// table.
QVector<BlendEntry> allowedBlendModes(const PictureView* view);

// Rebuild `combo` when the allowed list or an unavailable current key changes,
// disable an appended unavailable current entry, and return that key's index.
int syncBlendCombo(QComboBox* combo, const PictureView* view, const QString& currentKey);

// `key`'s position in engine order, or -1 when it is not a layer mode.
int blendModeIndex(const QString& key);

// The display name for PSD key `key`; `key` itself when it is not a layer mode.
QString blendModeName(const QString& key);

} // namespace pictura
