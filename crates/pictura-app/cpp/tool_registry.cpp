#include "tool_registry.h"

#include "tools.h"

#include <utility>

namespace pictura {

static_assert(static_cast<int>(ToolId::SingleColumnMarquee) + 1 == ToolRegistry::kToolCount,
              "ToolId catalogue size changed; update ToolRegistry::kToolCount");

void ToolRegistry::registerTool(ToolId id, std::unique_ptr<ToolHandler> handler)
{
    const int index = static_cast<int>(id);
    if (index < 0 || index >= kToolCount) {
        return;
    }
    handlers_[index] = std::move(handler);
}

ToolHandler* ToolRegistry::forTool(ToolId id) const
{
    const int index = static_cast<int>(id);
    if (index < 0 || index >= kToolCount) {
        return nullptr;
    }
    return handlers_[index].get();
}

} // namespace pictura
