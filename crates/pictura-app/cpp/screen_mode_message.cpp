// The Full Screen Mode advisory CS6 shows when entering a full-screen mode from
// Standard, with the "Don't show again" choice persisted in the session store.

#include "screen_mode_message.h"

#include "session.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {

bool confirmFullScreenMode(QWidget* parent)
{
    SessionState session = loadSession();
    if (!session.showFullScreenMessage) {
        return true;
    }
    QMessageBox box(QMessageBox::NoIcon, QObject::tr("Message"),
                    QObject::tr("In Full Screen Mode, panels are hidden. They can be accessed "
                                "on the sides of the screen, or revealed by pressing Tab.\n\n"
                                "While in Full Screen Mode, you can return to Standard Screen "
                                "Mode by pressing 'F' or Esc."),
                    QMessageBox::NoButton, parent);
    box.setObjectName(QStringLiteral("fullScreenMessage"));
    QPushButton* proceed = box.addButton(QObject::tr("Full Screen"), QMessageBox::AcceptRole);
    box.addButton(QObject::tr("Cancel"), QMessageBox::RejectRole);
    box.setDefaultButton(proceed);
    auto* dontShow = new QCheckBox(QObject::tr("Don't show again"), &box);
    dontShow->setObjectName(QStringLiteral("fullScreenDontShowAgain"));
    box.setCheckBox(dontShow);
    box.exec();
    if (box.clickedButton() != proceed) {
        return false;
    }
    if (dontShow->isChecked()) {
        session = loadSession();
        session.showFullScreenMessage = false;
        saveSession(session);
    }
    return true;
}

} // namespace pictura
