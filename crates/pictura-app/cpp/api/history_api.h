#pragma once

#include <QtCore/QMetaObject>
#include <QtCore/QString>

#include <functional>

namespace pictura {
class PictureView;
}

namespace pictura::history_api {

bool undo(PictureView& view);
bool redo(PictureView& view);
bool canUndo(const PictureView& view);
bool canRedo(const PictureView& view);
int depth(const PictureView& view);
int count(const PictureView& view);
int index(const PictureView& view);
QString label(const PictureView& view, int i);
bool jump(PictureView& view, int i);
bool addSnapshot(PictureView& view, const QString& label);
int snapshotCount(const PictureView& view);
QString snapshotLabel(const PictureView& view, int i);
bool restoreSnapshot(PictureView& view, int i);

/// History Brush source row: the state index, or -1 when the source is a
/// snapshot or a dropped state.
int brushSourceState(const PictureView& view);
int brushSourceSnapshot(const PictureView& view);
bool setBrushSource(PictureView& view, bool snapshot, int index);

bool purgeUndo(PictureView& view);
bool purgeHistories(PictureView& view);
bool purgeAll(PictureView& view);

/// Apply the capture-affecting History Options to `view`.
void setHistoryOptions(PictureView& view, bool allowNonLinear, bool visibilityUndoable,
                       bool autoSnapshotOnSave);

/// Create a new document in `target` from state or named snapshot `index` of
/// `source`, with one `Duplicate State` history entry.
bool newDocumentFromState(const PictureView& source, PictureView& target, bool snapshot,
                          int index);

/// Connect `PictureView::changed` without the consumer including the generated
/// bridge header just for the signal.
QMetaObject::Connection connectChanged(PictureView& view, const QObject* context,
                                       const std::function<void()>& fn);

} // namespace pictura::history_api