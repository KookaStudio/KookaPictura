#pragma once

#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtCore/QtGlobal>

namespace pictura {
class PictureView;
}

namespace pictura::document_api {

/// One pyramid level's document-space size; invalid without a document or past
/// the last level.
struct CanvasSize {
    int width = 0;
    int height = 0;
    bool valid = false;
};

bool open(PictureView& view, const QString& path);
bool revert(PictureView& view, const QString& path);
bool openImage(PictureView& view, const QString& path);
bool openAsSmartObject(PictureView& view, const QString& path);
bool newDocument(PictureView& view, int width, int height, const QString& mode, int depth,
                 const QString& background);
/// File > Scripts > Load Files into Stack: build a new document in `view` with
/// one layer per decodable path. Returns the paths that could not be loaded.
QStringList loadFilesIntoStack(PictureView& view, const QStringList& paths, bool align,
                               bool smartObject);
bool save(PictureView& view, const QString& path);
/// Write a copy without recording the path or clearing the modified state.
bool saveCopy(PictureView& view, const QString& path);
// Native PSD/PSB save off the GUI thread. `saveStart` is false for every other
// suffix, including the engine Save As formats; `saveFinish` joins and applies
// the path/dirty result.
bool saveStart(PictureView& view, const QString& path);
/// Native PSD/PSB copy off the GUI thread; `saveFinish` skips the bookkeeping.
bool saveCopyStart(PictureView& view, const QString& path);
bool saveRunning(const PictureView& view);
bool saveFinish(PictureView& view);
void saveSetDelayForTest(PictureView& view, quint64 ms);
bool isDirty(const PictureView& view);
QString filePath(const PictureView& view);
/// Post-open normalization notices joined for the status bar; empty when none.
QString openNotice(const PictureView& view);
QString documentMode(const PictureView& view);
int documentDepthBits(const PictureView& view);

/// Every pyramid level's size in one crossing (index = level).
QVector<CanvasSize> pyramidSizes(const PictureView& view);
/// One level's size from the full-extent crossing.
CanvasSize pyramidSize(const PictureView& view, int level);
int levelCount(const PictureView& view);

} // namespace pictura::document_api