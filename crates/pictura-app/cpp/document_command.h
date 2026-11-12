#pragma once

#include <QtCore/QString>

#include <functional>

class QWidget;

namespace pictura {

class PictureView;

// Run a whole-document command (#1278): Image Size, Canvas Size, Rotate/Flip,
// a mode or depth conversion, Merge or Flatten. `command` calls its bridge
// function with the view armed; on a large document the engine then runs as a
// cancellable job with a progress window titled `title`. True when the command
// applied; false when it was refused or cancelled, leaving the document and
// history unchanged. `document_op_result` names a merged layer afterwards.
bool runDocumentCommand(QWidget* parent, PictureView& view, const QString& title,
                        const std::function<void()>& command);

} // namespace pictura
