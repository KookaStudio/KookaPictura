#pragma once
#include "transform_bridge.h"
#include <QtCore/QtGlobal>
#include <QtGui/QImage>

namespace pictura::content_aware_test {
inline void holdJobs(PictureView& view, bool hold) { content_aware_test_hold(view, hold); }
inline void releaseJob(PictureView& view, quint64 generation) { content_aware_test_release(view, generation); }
inline bool jobStarted(PictureView& view, quint64 generation) { return content_aware_test_started(view, generation); }
inline bool jobFinished(PictureView& view, quint64 generation) {
    if (!content_aware_test_finished(view, generation)) return false;
    content_aware_poll(view);
    return true;
}
inline quint64 currentGeneration(PictureView& view) { return content_aware_test_generation(view); }
inline QImage previewImage(PictureView& view) { return view.move_preview_layer(); }
}
