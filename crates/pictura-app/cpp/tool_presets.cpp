#include "tool_presets.h"

#include "paint_options.h"
#include "tool_options_json.h"

namespace pictura {

QString toolPresetToolName(ToolId id)
{
    if (id == ToolId::Brush) {
        return QStringLiteral("paintbrushTool");
    }
    if (id == ToolId::Pencil) {
        return QStringLiteral("pencilTool");
    }
    return toolIdName(id);
}

ToolId toolPresetToolId(const QJsonObject& values, bool* ok)
{
    QString name = values.value(QStringLiteral("toolId")).toString();
    if (name.isEmpty()) {
        name = values.value(QStringLiteral("tool")).toString();
        if (name == QStringLiteral("paintbrushTool")) {
            name = QStringLiteral("brush");
        } else if (name == QStringLiteral("pencilTool")) {
            name = QStringLiteral("pencil");
        }
    }
    for (ToolId id : allToolIds()) {
        if (toolIdName(id) == name) {
            if (ok) {
                *ok = true;
            }
            return id;
        }
    }
    if (ok) {
        *ok = false;
    }
    return ToolId::Move;
}

bool toolPresetMatches(const QJsonObject& values, ToolId id)
{
    bool ok = false;
    return toolPresetToolId(values, &ok) == id && ok;
}

void applyToolPreset(ToolController& tools, const QJsonObject& values)
{
    const QJsonValue options = values.value(QStringLiteral("options"));
    if (options.isObject()) {
        patchToolOptionsJson(tools, options.toObject());
        return;
    }
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    if (!paint) {
        return;
    }
    if (const QJsonValue opacity = values.value(QStringLiteral("opacity")); opacity.isDouble()) {
        paint->setOpacity(opacity.toInt());
    }
    if (const QJsonValue flow = values.value(QStringLiteral("flow")); flow.isDouble()) {
        paint->setFlow(flow.toInt());
    }
    if (const QJsonValue size = values.value(QStringLiteral("size")); size.isDouble()) {
        paint->setSize(size.toInt());
    }
}

} // namespace pictura
