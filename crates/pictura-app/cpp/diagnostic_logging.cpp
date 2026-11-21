#include "diagnostic_logging.h"

#include <QtCore/QByteArray>
#include <QtCore/QDir>

#include <cstdio>

#include "api/engine_bridge.h"

namespace pictura {
namespace {

void logQt(QtMsgType type, const QMessageLogContext& context, const QString& message)
{
    switch (type) {
    case QtDebugMsg:
    case QtInfoMsg:
    case QtWarningMsg:
    case QtCriticalMsg:
    case QtFatalMsg:
        break;
    default:
        return;
    }
    const bool fatal = type == QtFatalMsg;
    const int level = type == QtDebugMsg ? 0 : type == QtInfoMsg ? 1
                    : type == QtWarningMsg ? 2 : 3;
    const QByteArray category = QString::fromUtf8(context.category != nullptr
                                                  ? context.category : "default").toUtf8();
    if (fatal) {
        std::fprintf(stderr, "qt[ERROR fatal]: %s\n", qPrintable(message));
    } else {
        std::fprintf(stderr, "qt[%d]: %s\n", static_cast<int>(type), qPrintable(message));
    }
    std::fflush(stderr);
    diagnostics_qt(level, ::rust::Str(category.constData(), static_cast<size_t>(category.size())), fatal);
}

} // namespace

DiagnosticLogging::DiagnosticLogging()
    : previous_(qInstallMessageHandler(logQt))
{
}

DiagnosticLogging::DiagnosticLogging(const QString& stateHome, bool file, bool stallForTest)
    : DiagnosticLogging()
{
    start(stateHome, file, stallForTest);
}

DiagnosticLogging::~DiagnosticLogging()
{
    qInstallMessageHandler(previous_);
    if (started_) {
        diagnostics_stop();
    }
}

void DiagnosticLogging::start(const QString& stateHome, bool file, bool stallForTest)
{
    if (started_) {
        return;
    }
    const QString root = stateHome.isEmpty() ? QDir::homePath() + QStringLiteral("/.local/state")
                                           : stateHome;
    const QString path = file ? QDir(root).filePath(QStringLiteral("kooka-pictura/logs"))
                              : QString();
    diagnostics_start(path, stallForTest);
    started_ = true;
}

} // namespace pictura
