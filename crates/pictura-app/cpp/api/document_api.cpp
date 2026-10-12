#include "api/document_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <rust/cxx.h>

namespace pictura::document_api {

bool open(PictureView& view, const QString& path)
{
    return view.open(path);
}

bool openImage(PictureView& view, const QString& path)
{
    return view.open_image(path);
}

bool openAsSmartObject(PictureView& view, const QString& path)
{
    return view.open_as_smart_object(path);
}

bool newDocument(PictureView& view, int width, int height, const QString& mode, int depth,
                 const QString& background)
{
    return view.new_document(width, height, mode, depth, background);
}

bool save(PictureView& view, const QString& path)
{
    return view.save(path);
}

bool isDirty(const PictureView& view)
{
    return view.is_dirty();
}

QString filePath(const PictureView& view)
{
    return view.file_path();
}

QString openNotice(const PictureView& view)
{
    return view.open_notice();
}

QString documentMode(const PictureView& view)
{
    return view.document_mode();
}

int documentDepthBits(const PictureView& view)
{
    return view.document_depth_bits();
}

QVector<CanvasSize> pyramidSizes(const PictureView& view)
{
    const ::rust::Vec<int32_t> extents = view.display_level_extents();
    QVector<CanvasSize> sizes;
    sizes.reserve(int(extents.size() / 2));
    for (size_t i = 0; i + 1 < extents.size(); i += 2) {
        sizes.append(CanvasSize{extents[i], extents[i + 1], true});
    }
    return sizes;
}

CanvasSize pyramidSize(const PictureView& view, int level)
{
    const QVector<CanvasSize> sizes = pyramidSizes(view);
    if (level < 0 || level >= sizes.size()) {
        return CanvasSize{};
    }
    return sizes.at(level);
}

int levelCount(const PictureView& view)
{
    return int(view.display_level_extents().size() / 2);
}

} // namespace pictura::document_api