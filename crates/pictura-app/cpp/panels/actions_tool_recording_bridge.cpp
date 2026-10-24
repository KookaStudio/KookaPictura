#include "panels/actions_tool_recording_bridge.h"

namespace pictura {

namespace {

ToolRecordingActiveHook g_activeHook = nullptr;
ToolRecordingRecordHook g_recordHook = nullptr;
ToolRecordingRefusedHook g_refusedHook = nullptr;

} // namespace

void set_tool_recording_hooks(ToolRecordingActiveHook active, ToolRecordingRecordHook record,
                              ToolRecordingRefusedHook refused)
{
    g_activeHook = active;
    g_recordHook = record;
    g_refusedHook = refused;
}

bool active_tool_recording()
{
    return g_activeHook && g_activeHook();
}

bool record_active_tool_stroke(const QString& payload)
{
    return g_recordHook && g_recordHook(payload);
}

void tool_stroke_refused(const QString& message)
{
    if (g_refusedHook) {
        g_refusedHook(message);
    }
}

} // namespace pictura
