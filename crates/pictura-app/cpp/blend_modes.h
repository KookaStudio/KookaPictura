#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>

namespace pictura {

// One layer blend mode as the engine lists it (`BlendMode::LAYER_MODES`).
struct BlendEntry {
    QString key;
    QString name;
    bool hasNeutral = false;
};

// The 27 layer modes in engine order, read once from the engine. Shared by the
// Layers panel's blend combo and Mode filter, the New Layer dialog, the Layer
// Style dialog, and the Properties panel; a combo index is the engine index.
const QVector<BlendEntry>& blendModes();

// The display name for PSD key `key`; `key` itself when it is not a layer mode.
QString blendModeName(const QString& key);

} // namespace pictura
