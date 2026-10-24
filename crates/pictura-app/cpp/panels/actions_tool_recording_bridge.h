#pragma once

#include <QtCore/QString>

namespace pictura {

using ToolRecordingActiveHook = bool (*)();
using ToolRecordingRecordHook = bool (*)(const QString&);
using ToolRecordingRefusedHook = void (*)(const QString&);

// The C++ shell installs the live Actions-engine hooks here. The definitions
// live in a small TU compiled into the Rust static library so Rust unit tests
// and the Qt executable link the same symbols.
void set_tool_recording_hooks(ToolRecordingActiveHook active, ToolRecordingRecordHook record,
                              ToolRecordingRefusedHook refused);
bool active_tool_recording();
bool record_active_tool_stroke(const QString& payload);
void tool_stroke_refused(const QString& message);

} // namespace pictura
