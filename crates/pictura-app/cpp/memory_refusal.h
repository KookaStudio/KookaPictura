#pragma once

// The status-bar message of an edit the memory governor refused (#1211).

#include <QtCore/QString>

#include "api/paint_bridge.h"

class QWidget;

namespace pictura {

// "Could not <action>: not enough memory. …"
QString memoryRefusalMessage(const QString& action);

// "Could not <action>: image data could not be read back from the scratch
// disk. …" (#1217)
QString scratchRefusalMessage(const QString& action);
QString scratchFullRefusalMessage(const QString& action);

// The message for the typed `Refusal` the bridge reported, using `action` for
// the command: a memory shortfall, full scratch disks, or image data that could
// not be read back. `Refusal::None` yields no message.
QString refusalMessage(Refusal reason, const QString& action);

// As above with per-command copy: `memory` when the room could not hold the
// edit, `scratchFull` when the scratch disks are full, `unreadable` when image
// data could not be read back. `Refusal::None` yields no message.
QString refusalMessage(Refusal reason, const QString& memory, const QString& scratchFull,
                       const QString& unreadable);

// The typed memory or scratch-full message for a refused geometry commit.
// Other refusal reasons yield no message.
QString commitRefusalMessage(const QString& action);

// Show refusalMessage(reason, action) on the status bar of the main window that
// holds `within`; returns whether a refusal was presented.
bool reportRefusal(QWidget* within, Refusal reason, const QString& action);

// Show a typed memory or scratch-full commit refusal on the status bar of the
// main window that holds `within`; returns whether it was.
bool reportCommitRefusal(QWidget* within, const QString& action);

// Show the last refusal's message on the status bar of the main window that
// holds `within` (a panel may float in a window of its own) when the last gated
// edit was refused; returns whether it was.
bool reportMemoryRefusal(QWidget* within, const QString& action);

} // namespace pictura
