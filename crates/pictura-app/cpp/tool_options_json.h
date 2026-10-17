#pragma once

#include <QtCore/QJsonObject>

namespace pictura {

class ToolController;

// Every tool's options-bar settings as one JSON object for the session store,
// keyed by option group ("move", "type", "brush", "blur", …), so they survive
// a restart. The Crop options keep their own session keys, and the Clone
// Source slots (points in an open document) are not settings.
QJsonObject toolOptionsToJson(const ToolController& tools);

// Restore `json` into the groups without notifying; a missing group or field
// keeps its default, and each value is clamped as its setter would.
void applyToolOptionsJson(ToolController& tools, const QJsonObject& json);

} // namespace pictura
