#pragma once

namespace pictura {

// Install the application-wide filter that lets Enter commit a focused value
// input (numeric field, spin box, editable combo, dialog line edit) and
// deactivate it, so the key never reaches a tool or a dialog's default button.
// Idempotent; needs a live QApplication.
void installValueInputEnter();

} // namespace pictura
