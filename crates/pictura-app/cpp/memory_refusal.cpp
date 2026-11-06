#include "memory_refusal.h"

#include <QtCore/QCoreApplication>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QStatusBar>

#include "api/paint_bridge.h"

namespace pictura {

QString memoryRefusalMessage(const QString& action)
{
    return QCoreApplication::translate("pictura",
                                       "Could not %1: not enough memory. Close other documents "
                                       "or free memory, then try again.")
        .arg(action);
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

bool reportMemoryRefusal(QWidget* within, const QString& action)
{
    if (!edit_refused_for_memory()) {
        return false;
    }
    if (QMainWindow* main = mainWindowOf(within)) {
        main->statusBar()->showMessage(memoryRefusalMessage(action), 6000);
    }
    return true;
}

} // namespace pictura
