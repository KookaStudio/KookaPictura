#include "control_scope.h"

#include <QtCore/QEvent>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMenu>

namespace pictura {

ControlScope::ControlScope()
{
    qApp->installEventFilter(this);
}

ControlScope::~ControlScope()
{
    qApp->removeEventFilter(this);
}

bool ControlScope::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() != QEvent::Show) {
        return QObject::eventFilter(watched, event);
    }
    auto* dialog = qobject_cast<QDialog*>(watched);
    auto* menu = qobject_cast<QMenu*>(watched);
    // `jobBusy` is runBusyJob's progress window: it closes itself when the job
    // lands, and cancelling it would not cancel the job.
    if ((!dialog && !menu) || watched->objectName() == QStringLiteral("jobBusy")) {
        return QObject::eventFilter(watched, event);
    }
    auto* widget = static_cast<QWidget*>(watched);
    if (!cancelledAny_) {
        cancelledAny_ = true;
        cancelled_ = menu ? menu->title() : widget->windowTitle();
    }
    // Queued, because a modal's exec() has not started its loop at Show. The
    // context object drops the call if the widget is destroyed first.
    QTimer::singleShot(0, widget, [dialog, menu] {
        if (dialog) {
            dialog->reject();
        } else {
            menu->close();
        }
    });
    return QObject::eventFilter(watched, event);
}

} // namespace pictura
