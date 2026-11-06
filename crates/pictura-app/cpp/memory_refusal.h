#pragma once

// The status-bar message of an edit the memory governor refused (#1211).

#include <QtCore/QString>

class QWidget;

namespace pictura {

// "Could not <action>: not enough memory. …"
QString memoryRefusalMessage(const QString& action);

// Show memoryRefusalMessage(action) on the status bar of the main window that
// holds `within` (a panel may float in a window of its own) when the last
// gated edit was refused for memory; returns whether it was.
bool reportMemoryRefusal(QWidget* within, const QString& action);

} // namespace pictura
