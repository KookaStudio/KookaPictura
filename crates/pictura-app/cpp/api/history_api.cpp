#include "api/history_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/purge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

namespace pictura::history_api {

bool undo(PictureView& view)
{
    return view.undo();
}

bool redo(PictureView& view)
{
    return view.redo();
}

bool canUndo(const PictureView& view)
{
    return view.can_undo();
}

bool canRedo(const PictureView& view)
{
    return view.can_redo();
}

int depth(const PictureView& view)
{
    return view.history_depth();
}

int count(const PictureView& view)
{
    return view.history_count();
}

int index(const PictureView& view)
{
    return view.history_index();
}

QString label(const PictureView& view, int i)
{
    return view.history_label(i);
}

bool jump(PictureView& view, int i)
{
    return view.history_jump(i);
}

bool addSnapshot(PictureView& view, const QString& label)
{
    return view.history_add_snapshot(label);
}

int snapshotCount(const PictureView& view)
{
    return view.history_snapshot_count();
}

QString snapshotLabel(const PictureView& view, int i)
{
    return view.history_snapshot_label(i);
}

bool restoreSnapshot(PictureView& view, int i)
{
    return view.history_restore_snapshot(i);
}

int brushSourceState(const PictureView& view)
{
    return history_brush_source_state(view);
}

int brushSourceSnapshot(const PictureView& view)
{
    return history_brush_source_snapshot(view);
}

bool setBrushSource(PictureView& view, bool snapshot, int index)
{
    return set_history_brush_source(view, snapshot, index);
}

bool purgeUndo(PictureView& view)
{
    return purge_history(view);
}

bool purgeHistories(PictureView& view)
{
    return purge_named_history(view);
}

bool purgeAll(PictureView& view)
{
    return purge_all(view);
}

QMetaObject::Connection connectChanged(PictureView& view, const QObject* context,
                                       const std::function<void()>& fn)
{
    return QObject::connect(&view, &PictureView::changed, context, [fn]() { fn(); });
}

} // namespace pictura::history_api