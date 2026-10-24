// The document canvases: each window's canvas and its connections, the
// document-wide pushes to every window of a document, the view-pyramid level
// provider, and the canvas background colour.

#include "frame_canvas.h"

#include "frame_includes.h"

#include "api/document_api.h"
#include "api/paint_api.h"
#include "file_drop_router.h"
#include "pictura_debug_timing.h"

#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"

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

// Create the canvas for `window` and show its document's current image (or the
// view's fallback image). A loading window's canvas host is added to its
// placeholder page by `finishLoadingDocument`.
void PicturaMainWindow::setupCanvas(DocWindow& window)
{
    PictureView* view = window.doc->view;
    window.canvas = new ImageView(this);
    window.tip = new BrushTipPreview(window.canvas);
    window.tip->move(BrushTipPreview::kMargin, BrushTipPreview::kMargin);
    window.host = new CanvasScrollBars(this);
    window.host->setView(window.canvas);
    // A press on the canvas activates its window (see eventFilter).
    window.canvas->installEventFilter(this);
    if (fileDropRouter_) {
        window.canvas->setAcceptDrops(true);
        window.canvas->installEventFilter(fileDropRouter_);
    }
    if (view->has_document()) {
        window.canvas->setDocument(QSize(view->document_width(), view->document_height()));
        window.canvas->setProperty(kFrameRevisionKey, quint64(pictura::frame_revision(*view)));
    } else {
        window.canvas->replaceImage(view->image());
    }
    window.canvas->setCanvasColor(canvasColorForIndex(canvasColorIndex_));
    window.canvas->setFreePan(freePan_);
    window.canvas->setSnapToWorkspace(snapCanvasToWorkspace_);
    prepareCanvasGuides(window);

    // Present from the view pyramid: the canvas crops a level instead of
    // scaling the full-resolution image.
    wireCanvasLevelProvider(view, window.canvas);
}

// Connect a ready window's canvas to the frame. The canvas is the sender, so
// the connections end with the window.
void PicturaMainWindow::wireCanvas(DocWindow& window)
{
    connect(window.canvas, &ImageView::zoomChanged, this,
            &PicturaMainWindow::onCanvasZoomChanged);
    connect(window.canvas, &ImageView::mouseMoved, this, [this](const QPointF& p) {
        if (infoPanel_) {
            infoPanel_->setCursorPosition(p);
        }
    });
    // Scroll All Windows: a Hand drag pans the other visible windows too.
    connect(window.canvas, &ImageView::handPanned, this,
            [this, canvas = window.canvas](const QPointF& delta, Qt::KeyboardModifiers mods) {
                scrollOtherWindows(canvas, delta, mods);
            });
    // The active tool's canvas context menu (or the pasteboard colour menu).
    connect(window.canvas, &ImageView::contextMenuRequested, this,
            [this, canvas = window.canvas](const QPoint& globalPos) {
                showCanvasContextMenu(canvas, globalPos);
            });
}

// Connect a ready document's view to the frame, once for all its windows.
void PicturaMainWindow::wireDocument(DocEntry& doc)
{
    PictureView* view = doc.view;
    connect(view, &PictureView::changed, this, &PicturaMainWindow::refresh);
    connect(view, &PictureView::sessionCancelled, this, [this]() {
        if (tools_) {
            tools_->onSessionCancelled();
        }
    });
    trackRecovery(view);
    // The windows are looked up on each blit, so one opened or closed since
    // is reached or skipped.
    connect(view, &PictureView::regionBlitted, this,
            [this, view](const QImage& region, int x, int y) {
                pictura::ScopedTimer blitSlotTimer("cxx_regionBlitted_slot");
                const int index = indexOfView(view);
                if (index >= 0) {
                    forEachCanvas(*docs_.at(index), [&region, x, y](ImageView* canvas) {
                        canvas->blitRegion(region, x, y);
                    });
                }
                // The cheapest sync only: restart the same 120 ms panel timer the
                // `changed` path uses, and never call image()/replaceImage(), so
                // the region path cannot refresh panels faster than a full
                // recomposite.
                panelRefreshTimer_->start();
                // A region update can change the active layer's visibility
                // without a `changed` emission; keep the tool cursor in sync.
                if (tools_) {
                    tools_->refreshCursor();
                }
                // Mid-stroke, skip the command-registry and title fan-out on
                // every dab (Krita's "unnecessary objects per event"). Releasing
                // the stroke emits `changed`, whose refresh() runs them once.
                if (view->is_painting()) {
                    // A present arms its own flush: the next event-loop turn
                    // folds every dab that arrived since into one region, so the
                    // composite and the present track the frame rate, not the
                    // input rate.
                    paint_api::presentDeferred(*view);
                    return;
                }
                updateTabTitle(index);
                updateWindowTitle();
                if (registry_) {
                    registry_->refresh();
                }
            });
}

void PicturaMainWindow::forEachCanvas(const DocEntry& doc,
                                      const std::function<void(ImageView*)>& fn) const
{
    for (const DocWindow* window : doc.windows) {
        if (window->canvas) {
            fn(window->canvas);
        }
    }
}

QList<ImageView*> PicturaMainWindow::activeDocumentCanvases() const
{
    QList<ImageView*> canvases;
    if (!activeWindow_ || !activeWindow_->canvas) {
        return canvases;
    }
    canvases.append(activeWindow_->canvas);
    forEachCanvas(*activeWindow_->doc, [&canvases](ImageView* canvas) {
        if (!canvases.contains(canvas)) {
            canvases.append(canvas);
        }
    });
    return canvases;
}

namespace {

constexpr int kCanvasColorCount = 4;
// Space+F cycles the pane colours; index 0 is a sentinel resolved to the
// theme's current workspace shade so an open canvas keeps matching the empty
// pane across brightness changes.
const QColor kCanvasColors[kCanvasColorCount] = {
    QColor(), QColor(82, 82, 82), QColor(0, 0, 0), QColor(255, 255, 255)};
// The names the canvas-background menu shows, one per `kCanvasColors` entry.
const char* const kCanvasColorNames[kCanvasColorCount] = {
    QT_TR_NOOP("Custom"), QT_TR_NOOP("Dark Gray"), QT_TR_NOOP("Black"),
    QT_TR_NOOP("White")};

} // namespace

QColor PicturaMainWindow::canvasColorForIndex(int index) const
{
    return index <= 0 ? Theme::workspaceColor() : kCanvasColors[index];
}

void PicturaMainWindow::cycleCanvasColor(bool forward)
{
    canvasColorIndex_ =
        (canvasColorIndex_ + (forward ? 1 : -1) + kCanvasColorCount) % kCanvasColorCount;
    for (const DocWindow* window : windows_) {
        if (window->canvas) {
            window->canvas->setCanvasColor(canvasColorForIndex(canvasColorIndex_));
        }
    }
}

QMenu* PicturaMainWindow::canvasBackgroundMenu()
{
    auto* menu = new QMenu(this);
    for (int i = 0; i < kCanvasColorCount; ++i) {
        QAction* action = menu->addAction(tr(kCanvasColorNames[i]));
        connect(action, &QAction::triggered, this, [this, i]() {
            canvasColorIndex_ = i;
            const QColor chosen = canvasColorForIndex(i);
            for (const DocWindow* window : windows_) {
                if (window->canvas) {
                    window->canvas->setCanvasColor(chosen);
                }
            }
        });
    }
    return menu;
}

void PicturaMainWindow::showCanvasBackgroundMenu(const QPoint& globalPos)
{
    if (!backgroundMenu_) {
        backgroundMenu_ = canvasBackgroundMenu();
    }
    backgroundMenu_->popup(globalPos);
}

void PicturaMainWindow::applyWorkspaceCanvasColor()
{
    if (screenMode_ != ScreenMode::Standard) {
        return;
    }
    const QColor color = canvasColorForIndex(canvasColorIndex_);
    for (const DocWindow* window : windows_) {
        if (window->canvas) {
            window->canvas->setCanvasColor(color);
        }
    }
}

} // namespace pictura
