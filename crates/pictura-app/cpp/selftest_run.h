#pragma once

#include "selftest_fixture.h"
#include "selftest_report.h"
#include "tools.h"

namespace pictura::selftest {

class SelfTestRun {
public:
    explicit SelfTestRun(PicturaMainWindow& frame, SelfTestReport& report = selfTest());
    ~SelfTestRun();
    SelfTestRun(const SelfTestRun&) = delete;
    SelfTestRun& operator=(const SelfTestRun&) = delete;

private:
    SelfTestReport::RunGuard reportGuard_;
    selftest_fixture::DocumentCleanup documents_;
    ToolId previousTool_ = ToolId::Move;
};

}  // namespace pictura::selftest
