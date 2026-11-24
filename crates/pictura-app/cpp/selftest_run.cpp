#include "selftest_run.h"

#include "dialogs.h"

namespace pictura::selftest {

SelfTestRun::SelfTestRun(PicturaMainWindow& frame, SelfTestReport& report)
    : reportGuard_(report)
    , documents_(frame)
{
    previousTool_ = documents_.frame.activeTool();
    setUnsavedPromptInteractive(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    setTransformPromptInteractive(false);
    setCropPromptInteractive(false);
}

SelfTestRun::~SelfTestRun()
{
    documents_.frame.setActiveTool(previousTool_);
    documents_.closeAll();
}

}  // namespace pictura::selftest
