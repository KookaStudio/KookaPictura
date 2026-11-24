#include "selftest_report.h"

#include "selftest_checks.h"

#include <cstdio>
#include <cstring>

namespace pictura {

SelfTestReport& selfTest() {
    static SelfTestReport instance;
    return instance;
}

void SelfTestReport::setSink(std::FILE* sink) {
    sink_ = sink ? sink : stderr;
}

void SelfTestReport::emitToken(const char* keyword, const char* fmt, va_list ap) {
    std::fprintf(sink_, "pictura self-test: %s %s %s", keyword, suite_ ? suite_ : selftest::kLegacySuite, name_);
    if (fmt && *fmt) {
        std::fputc(' ', sink_);
        std::vfprintf(sink_, fmt, ap);
    }
    std::fputc('\n', sink_);
    std::fflush(sink_);
}

void SelfTestReport::begin(const char* name) {
    begin(name, selftest::suiteForCheck(name ? name : ""));
}

void SelfTestReport::begin(const char* name, const char* suite) {
    if (name && *name) {
        std::snprintf(name_, sizeof(name_), "%s", name);
    } else {
        std::snprintf(name_, sizeof(name_), "%s", "unnamed");
    }
    const char* nextSuite = suite ? suite : selftest::kLegacySuite;
    if (!suite_ || std::strcmp(suite_, nextSuite) != 0) {
        suite_ = nextSuite;
        std::fprintf(sink_, "pictura self-test: SUITE %s %s\n", suite_, name_);
        std::fflush(sink_);
    }
}

void SelfTestReport::pass(const char* fmt, ...) {
    ++passed_;
    va_list ap;
    va_start(ap, fmt);
    emitToken("PASS", fmt, ap);
    va_end(ap);
}

void SelfTestReport::skip(const char* reason) {
    ++skipped_;
    std::fprintf(sink_, "pictura self-test: SKIP %s %s", suite_ ? suite_ : selftest::kLegacySuite, name_);
    if (reason && *reason) {
        std::fputc(' ', sink_);
        std::fputs(reason, sink_);
    }
    std::fputc('\n', sink_);
    std::fflush(sink_);
}

int SelfTestReport::failToken(int code, const char* fmt, va_list ap) {
    ++failed_;
    std::fprintf(sink_, "pictura self-test: FAIL %s %s %d", suite_ ? suite_ : selftest::kLegacySuite, name_, code);
    if (fmt && *fmt) {
        std::fputc(' ', sink_);
        std::vfprintf(sink_, fmt, ap);
    }
    std::fputc('\n', sink_);
    std::fflush(sink_);
    return code;
}

int SelfTestReport::fail(int code, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    const int result = failToken(code, fmt, ap);
    va_end(ap);
    return result;
}

int SelfTestReport::failRun(const char* name, int code, const char* fmt, ...) {
    begin(name);
    va_list ap;
    va_start(ap, fmt);
    const int result = failToken(code, fmt, ap);
    va_end(ap);
    finish();
    return result;
}

int SelfTestReport::finish() {
    if (!finished_) {
        finished_ = true;
        std::fprintf(sink_, "pictura self-test: SUMMARY passed=%d failed=%d skipped=%d\n",
                     passed_, failed_, skipped_);
        std::fflush(sink_);
    }
    return failed_ ? 1 : 0;
}

}  // namespace pictura
