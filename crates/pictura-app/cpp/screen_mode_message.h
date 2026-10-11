#pragma once

class QWidget;

namespace pictura {

// CS6's Full Screen Mode advisory: a "Message" box explaining that panels are
// hidden, that Tab reveals them, and that F or Esc returns to Standard Screen
// Mode, with a "Don't show again" checkbox persisted in the session store.
// Returns true to proceed with the mode change, false when cancelled.
bool confirmFullScreenMode(QWidget* parent);

} // namespace pictura
