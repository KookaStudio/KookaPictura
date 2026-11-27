#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>

#include "tools.h"

namespace pictura {

class ToolController;

// The owning-tool string stored in a Tools preset. Brush and Pencil keep the
// Photoshop class ids used by `.tpl`; every other tool uses its catalogue name.
QString toolPresetToolName(ToolId id);

// Resolve a preset row's `toolId` (or its `tool` class id) to a catalogue tool.
ToolId toolPresetToolId(const QJsonObject& values, bool* ok = nullptr);

bool toolPresetMatches(const QJsonObject& values, ToolId id);

// Apply a row's `options` snapshot, or the mapped `.tpl` opacity/flow/size when
// the row has no snapshot.
void applyToolPreset(ToolController& tools, const QJsonObject& values);

} // namespace pictura
