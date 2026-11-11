#pragma once

class QWidget;

namespace pictura {

class PictureView;

// Commit the active Free Transform session (#1278). A large commit runs as a
// cancellable job with a progress window; a small one commits at once. True
// when the commit recorded its history state. False when it was refused or
// cancelled; after a cancel the session is still active, its document and
// history unchanged.
bool runTransformCommit(QWidget* parent, PictureView& view);

} // namespace pictura
