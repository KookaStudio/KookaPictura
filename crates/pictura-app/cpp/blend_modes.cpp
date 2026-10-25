#include "blend_modes.h"

#include "pictura_app/src/cxxqt_object/engine_tables/blend_modes.cxxqt.h"

namespace pictura {

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

QString blendModeName(const QString& key)
{
    for (const BlendEntry& entry : blendModes()) {
        if (entry.key == key) {
            return entry.name;
        }
    }
    return key;
}

} // namespace pictura
