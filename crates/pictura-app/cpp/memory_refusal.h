#pragma once

// The status-bar message of an edit the memory governor refused (#1211).

#include <QtCore/QString>

class QWidget;

namespace pictura {

// "Could not <action>: not enough memory. …"
QString memoryRefusalMessage(const QString& action);

// "Could not <action>: image data could not be read back from the scratch
// disk. …" (#1217)
QString scratchRefusalMessage(const QString& action);

// "Could not <action>: the scratch disks are full. …" after a scratch-full
// refusal of a spilling commit (#1294), memoryRefusalMessage(action) otherwise.
QString commitRefusalMessage(const QString& action);

// Show commitRefusalMessage(action) on the status bar of the main window that
// holds `within` when the last gated edit was refused for memory; returns
// whether it was.
bool reportCommitRefusal(QWidget* within, const QString& action);

// Show memoryRefusalMessage(action), or scratchRefusalMessage(action), on the
// status bar of the main window that holds `within` (a panel may float in a
// window of its own) when the last gated edit was refused for memory or for
// tiles lost from the scratch disk; returns whether it was.
bool reportMemoryRefusal(QWidget* within, const QString& action);

} // namespace pictura
