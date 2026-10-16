#include "frame_canvas.h"

#include "api/document_api.h"
#include "frame.h"
#include "image_view.h"
#include "theme.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QPointer>
#include <QtCore/QStringList>

namespace pictura {

void wireCanvasLevelProvider(PictureView* view, ImageView* canvas)
{
    if (!view || !canvas) {
        return;
    }
    // Present from the view pyramid: the canvas crops a level instead of
    // scaling the full-resolution image. The lambdas stay valid via QPointer.
    QPointer<PictureView> levelView = view;
    canvas->setLevelProvider(ImageView::LevelProvider{
        [levelView](int level, int x, int y, int w, int h) {
            // The canvas display source: the solo mask plane while a mask is
            // soloed, else the composite. Other panels keep `display_image`.
            return levelView ? levelView->canvas_display_image(level, x, y, w, h) : QImage();
        },
        [levelView]() { return levelView ? document_api::levelCount(*levelView) : 0; },
        [levelView](int level) {
            if (!levelView) {
                return QSize();
            }
            const document_api::CanvasSize size = document_api::pyramidSize(*levelView, level);
            return size.valid ? QSize(size.width, size.height) : QSize();
        },
        [levelView]() { return levelView ? levelView->canvas_revision() : quint64(0); },
        [levelView]() { return levelView && levelView->is_painting(); },
        [levelView]() { return levelView ? levelView->preview_present_level() : 0; }});
}

namespace {

constexpr int kCanvasColorCount = 4;
// Space+F cycles the pane colours; index 0 is a sentinel resolved to the
// theme's current workspace shade so an open canvas keeps matching the empty
// pane across brightness changes.
const QColor kCanvasColors[kCanvasColorCount] = {
    QColor(), QColor(82, 82, 82), QColor(0, 0, 0), QColor(255, 255, 255)};

} // namespace

QColor PicturaMainWindow::canvasColorForIndex(int index) const
{
    return index <= 0 ? Theme::workspaceColor() : kCanvasColors[index];
}

void PicturaMainWindow::cycleCanvasColor(bool forward)
{
    canvasColorIndex_ =
        (canvasColorIndex_ + (forward ? 1 : -1) + kCanvasColorCount) % kCanvasColorCount;
    for (const DocEntry& entry : docs_) {
        if (entry.canvas) {
            entry.canvas->setCanvasColor(canvasColorForIndex(canvasColorIndex_));
        }
    }
}

void PicturaMainWindow::applyWorkspaceCanvasColor()
{
    if (screenMode_ != ScreenMode::Standard) {
        return;
    }
    const QColor color = canvasColorForIndex(canvasColorIndex_);
    for (const DocEntry& entry : docs_) {
        if (entry.canvas) {
            entry.canvas->setCanvasColor(color);
        }
    }
}

} // namespace pictura
