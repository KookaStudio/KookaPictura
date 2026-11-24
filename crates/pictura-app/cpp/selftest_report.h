#pragma once
#include <cstdarg>
#include <cstdio>

#include "selftest_codes.h"

namespace pictura {
class SelfTestReport {
public:
    void begin(const char* name);
    void begin(const char* name, const char* suite);
    void pass(const char* fmt = nullptr, ...);
    void skip(const char* reason);
    int  fail(int code, const char* fmt, ...);
    int  failRun(const char* name, int code, const char* fmt, ...);
    int  finish();
    void setSink(std::FILE* sink);
    int failedCount() const { return failed_; }
    int passedCount() const { return passed_; }
    int skippedCount() const { return skipped_; }

    struct RunGuard {
        SelfTestReport& report;
        explicit RunGuard(SelfTestReport& r) : report(r) {}
        ~RunGuard() { report.finish(); }
        RunGuard(const RunGuard&) = delete;
        RunGuard& operator=(const RunGuard&) = delete;
    };
private:
    void emitToken(const char* keyword, const char* fmt, va_list ap);
    int failToken(int code, const char* fmt, va_list ap);
    const char* suite_ = nullptr;
    bool finished_ = false;
    std::FILE* sink_ = stderr;
    char name_[160] = "unnamed";
    int passed_ = 0, failed_ = 0, skipped_ = 0;
};
SelfTestReport& selfTest();
}  // namespace pictura
#define ST_BEGIN(name) pictura::selfTest().begin(name)
#define ST_PASS(...) pictura::selfTest().pass(__VA_ARGS__)
#define ST_SKIP(reason) pictura::selfTest().skip(reason)
#define ST_FAIL(code, ...) return pictura::selfTest().fail((code), __VA_ARGS__)
#define ST_FINISH() return pictura::selfTest().finish()
