#pragma once

#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QVector>

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
bool openImage(PictureView& view, const QString& path);
bool openAsSmartObject(PictureView& view, const QString& path);
bool newDocument(PictureView& view, int width, int height, const QString& mode, int depth,
                 const QString& background);
bool save(PictureView& view, const QString& path);
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