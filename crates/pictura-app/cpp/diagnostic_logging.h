#pragma once

#include <QtCore/QString>
#include <QtCore/QtLogging>

namespace pictura {

class DiagnosticLogging {
public:
    DiagnosticLogging();
    DiagnosticLogging(const QString& stateHome, bool file, bool stallForTest = false);
    ~DiagnosticLogging();

    void start(const QString& stateHome, bool file, bool stallForTest = false);

    DiagnosticLogging(const DiagnosticLogging&) = delete;
    DiagnosticLogging& operator=(const DiagnosticLogging&) = delete;

private:
    QtMessageHandler previous_;
    bool started_ = false;
};

} // namespace pictura
