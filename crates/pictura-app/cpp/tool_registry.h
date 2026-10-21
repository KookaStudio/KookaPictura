#pragma once

#include "tool_handler.h"

#include <array>
#include <memory>

namespace pictura {

enum class ToolId;

// ToolId -> handler. Unregistered tools (most of the 71-row catalogue) return
// null so the controller falls through to the legacy switch.
class ToolRegistry {
public:
    // The ToolId catalogue size at the time of writing; the static_assert in
    // tool_registry.cpp turns an enum growth into a compile error.
    static constexpr int kToolCount = 71;

    void registerTool(ToolId id, std::unique_ptr<ToolHandler> handler);
    ToolHandler* forTool(ToolId id) const;

    template <typename F>
    void forEach(F&& f) const
    {
        for (const auto& handler : handlers_) {
            if (handler) {
                f(*handler);
            }
        }
    }

private:
    std::array<std::unique_ptr<ToolHandler>, kToolCount> handlers_{};
};

} // namespace pictura
