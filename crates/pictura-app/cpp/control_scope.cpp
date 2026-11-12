#include "control_scope.h"

#include <QtCore/QEvent>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMenu>

#include "dialogs.h"

namespace pictura {

ControlScope::ControlScope(const UnsavedChoice* unsavedChoice)
{
    qApp->installEventFilter(this);
    setRequestUnsavedChoice(unsavedChoice);
}

ControlScope::~ControlScope()
{
    setRequestUnsavedChoice(nullptr);
    qApp->removeEventFilter(this);
}

bool ControlScope::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() != QEvent::Show) {
        return QObject::eventFilter(watched, event);
    }
    auto* dialog = qobject_cast<QDialog*>(watched);
    auto* menu = qobject_cast<QMenu*>(watched);
    // A background job's progress window closes itself when the job lands:
    // `jobBusy` (runBusyJob) and `jobProgress` (runCancellableJob), whose
    // rejection would cancel the job the request started (#1290).
    const QString name = watched->objectName();
    if ((!dialog && !menu) || name == QStringLiteral("jobBusy")
        || name == QStringLiteral("jobProgress")) {
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
