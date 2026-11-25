#include "memory_refusal.h"

#include <QtCore/QCoreApplication>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QStatusBar>

namespace pictura {

QString memoryRefusalMessage(const QString& action)
{
    return QCoreApplication::translate("pictura",
                                       "Could not %1: not enough memory. Close other documents "
                                       "or free memory, then try again.")
        .arg(action);
}

QString scratchRefusalMessage(const QString& action)
{
    return QCoreApplication::translate("pictura",
                                       "Could not %1: image data could not be read back from "
                                       "the scratch disk. Check the scratch disks, then try "
                                       "again.")
        .arg(action);
}

QString scratchFullRefusalMessage(const QString& action)
{
    return QCoreApplication::translate("pictura",
                                       "Could not %1: the scratch disks are full. Free space "
                                       "on the scratch disks, then try again.")
        .arg(action);
}

QString refusalMessage(Refusal reason, const QString& memory, const QString& scratchFull,
                       const QString& unreadable)
{
    switch (reason) {
    case Refusal::Memory: return memory;
    case Refusal::ScratchFull: return scratchFull;
    case Refusal::ScratchUnreadable: return unreadable;
    case Refusal::None: break;
    }
    return {};
}

QString refusalMessage(Refusal reason, const QString& action)
{
    return refusalMessage(reason, memoryRefusalMessage(action), scratchFullRefusalMessage(action),
                          scratchRefusalMessage(action));
}

QString commitRefusalMessage(const QString& action)
{
    const Refusal reason = last_refusal();
    if (reason != Refusal::Memory && reason != Refusal::ScratchFull) {
        return {};
    }
    return refusalMessage(reason, action);
}

namespace {

QMainWindow* mainWindowOf(QWidget* within)
{
    if (auto* main = qobject_cast<QMainWindow*>(within ? within->window() : nullptr)) {
        return main;
    }
    for (QWidget* top : QApplication::topLevelWidgets()) {
        if (auto* main = qobject_cast<QMainWindow*>(top)) {
            return main;
        }
    }
    return nullptr;
}

} // namespace

bool reportRefusal(QWidget* within, Refusal reason, const QString& action)
{
    if (reason == Refusal::None) {
        return false;
    }
    if (QMainWindow* main = mainWindowOf(within)) {
        main->statusBar()->showMessage(refusalMessage(reason, action), 6000);
    }
    return true;
}

bool reportMemoryRefusal(QWidget* within, const QString& action)
{
    return reportRefusal(within, last_refusal(), action);
}

bool reportCommitRefusal(QWidget* within, const QString& action)
{
    const Refusal reason = last_refusal();
    if (reason != Refusal::Memory && reason != Refusal::ScratchFull) {
        return false;
    }
    return reportRefusal(within, reason, action);
}

} // namespace pictura
