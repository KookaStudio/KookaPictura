#pragma once

class QDialog;
class QWidget;

namespace pictura {

// Install the application-wide filter that gives every top-level Qt-drawn
// QDialog Kooka's frameless chrome (title bar, outline, move, edge resize) as
// it is first polished. Idempotent; needs a live QApplication.
void installDialogChrome();

// The dialog's drawn title bar, or nullptr when it carries no chrome (exempt,
// or not yet polished).
QWidget* dialogTitleBar(const QDialog* dialog);

} // namespace pictura
