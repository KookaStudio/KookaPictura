// The document canvases: each window's canvas and its connections, the
// document-wide pushes to every window of a document, the view-pyramid level
// provider, and the canvas background colour.

#include "frame_canvas.h"

#include "frame_includes.h"
#include "panels/history_panel.h"
#include "panels/info_panel.h"
#include "panels/layers_panel.h"

#include "import_save_coordinator.h"

#include "api/document_api.h"
#include "api/history_api.h"
#include "api/paint_api.h"
#include "file_drop_router.h"
#include "pictura_debug_timing.h"

#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/paint_bridge.h"

#include <QtCore/QStringList>
#include <QtCore/QTimer>

#include <memory>

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
        [levelView, canvas](int level, int x, int y, int w, int h) {
            return levelView ? pictura::hdr_canvas_image(*levelView, level, x, y, w, h,
                                                        canvas->hdrExposure()) : QImage();
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
        [levelView]() { return levelView ? pictura::preview_present_level(*levelView) : 0; },
        [levelView](int level, const QRect& docRect) {
            if (levelView) {
                levelView->set_presented_view(level, docRect.x(), docRect.y(),
                                              docRect.width(), docRect.height());
            }
        },
        [levelView, canvas](int level, const QRect& rect) {
            if (!levelView) {
                return CanvasFrameCrop{};
            }
            const int offset = canvas->hdrExposure();
            if (!offset) {
                return canvasFrameCrop(*levelView, level, rect);
            }
            return CanvasFrameCrop{pictura::hdr_canvas_image(*levelView, level, rect.x(), rect.y(),
                                                        rect.width(), rect.height(), offset), {}};
        },
        [levelView]() { return levelView && pictura::tiles_pending(*levelView); }});
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
    window.canvas->setFlickPanning(static_cast<FlickPanningMode>(flickPanning_),
                                  gpuCompute_ && gpuAvailable_);
    window.canvas->setSnapToWorkspace(snapCanvasToWorkspace_);
    prepareCanvasGuides(window);
    applyCanvasViewState(window);

    // Present from the view pyramid: the canvas crops a level instead of
    // scaling the full-resolution image.
    wireCanvasLevelProvider(view, window.canvas);
}

void PicturaMainWindow::applyFlickPanning()
{
    const bool available = gpuCompute_ && gpuAvailable_;
    if (preferencesDialog_) {
        preferencesDialog_->setFlickPanning(flickPanning_, available);
    }
    for (const DocEntry* entry : documents_->docs()) {
        forEachCanvas(*entry, [this, available](ImageView* canvas) {
            canvas->setFlickPanning(static_cast<FlickPanningMode>(flickPanning_), available);
        });
    }
}

// Connect a ready window's canvas to the frame. The canvas is the sender, so
// the connections end with the window.
void PicturaMainWindow::wireCanvas(DocWindow& window)
{
    connect(window.canvas, &ImageView::zoomChanged, this,
            &PicturaMainWindow::onCanvasZoomChanged);
    connect(window.canvas, &ImageView::mouseMoved, this, [this](const QPointF& p) {
        if (panels_->infoPanel()) {
            panels_->infoPanel()->setCursorPosition(p);
        }
    });
    // Scroll All Windows: a Hand drag pans the other visible windows too.
    connect(window.canvas, &ImageView::handPanned, this,
            [this, canvas = window.canvas](const QPointF& delta, Qt::KeyboardModifiers mods) {
                documents_->scrollOtherWindows(canvas, delta, mods);
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
    if (view->has_document() && panels_ && panels_->historyPanel()
        && panels_->historyPanel()->autoFirstSnapshot()
        && history_api::snapshotCount(*view) == 0 && history_api::count(*view) <= 1) {
        history_api::addSnapshot(*view, QStringLiteral("Snapshot 1"));
    }
    doc.selectionGeneration = view->selection_generation();
    connect(view, &PictureView::changed, this, [this, view]() {
        if (view != activeView()) {
            if (DocEntry* changed = documents_->docAt(indexOfView(view))) {
                refreshDocumentSelection(*changed);
            }
        }
        refresh();
    });
    struct ScaleNotice {
        bool invalidated = false;
        bool pending = false;
        bool shown = false;
    };
    const auto notice = std::make_shared<ScaleNotice>();
    notice->invalidated = measurement_scale_needs_recalibration(*view);
    const QString message = edition_is_extended()
        ? tr("Pixels changed. Recalibrate the measurement scale.")
        : tr("Pixels changed. Recalibrate the measurement scale in Extended.");
    const auto deliverNotice = [this, view, notice, message]() {
        if (view == activeView() && notice->pending) {
            statusBar()->showMessage(message, 8000);
            notice->pending = false;
            notice->shown = true;
        }
    };
    connect(view, &PictureView::changed, view, [this, view, notice, message, deliverNotice]() {
        const bool invalidated = measurement_scale_needs_recalibration(*view);
        if (invalidated && !notice->invalidated) {
            notice->pending = true;
        } else if (!invalidated) {
            notice->pending = false;
            if (notice->shown && statusBar()->currentMessage() == message) {
                statusBar()->clearMessage();
            }
            notice->shown = false;
        }
        notice->invalidated = invalidated;
        deliverNotice();
    });
    connect(documents_.get(), &DocumentRegistry::activeDocumentChanged, view,
            [this, view, notice, message, deliverNotice]() {
        if (view != activeView() && notice->shown) {
            if (statusBar()->currentMessage() == message) {
                statusBar()->clearMessage();
            }
            notice->shown = false;
        }
        if (view == activeView() && notice->pending) {
            QTimer::singleShot(0, view, deliverNotice);
        }
    });
    connect(view, &QObject::destroyed, this, [this, notice, message]() {
        if (notice->shown && statusBar()->currentMessage() == message) {
            statusBar()->clearMessage();
        }
    });
    // A notifying edit already refreshed the panels that subscribe to it. Scope
    // the marker to the active view so a background document's notification
    // cannot suppress the active panels' refresh.
    connect(view, &PictureView::changed, this, [this, view]() {
        panels_->setPanelsFreshlyBound(false);
        if (view == activeView()) {
            panels_->setPanelsRefreshedByChanged(true);
        }
    });
    connect(view, &PictureView::sessionCancelled, this, [this, view]() {
        if (tools_) {
            tools_->onSessionCancelled();
        }
        if (view == activeView()) {
            refresh();
            panels_->stopPanelRefreshTimer();
            panels_->refreshPanels();
        }
    });
    connect(view, &PictureView::historyRestored, this, [this]() {
        if (tools_) {
            tools_->onSessionCancelled();
        }
    });
    if (importSave_) {
        importSave_->trackRecovery(view);
    }
    // The cheapest sync only: restart the same 120 ms panel timer the `changed`
    // path uses, and never call image()/replaceImage(), so the region path
    // cannot refresh panels faster than a full recomposite.
    const auto afterPresent = [this, view](int index) {
        // A region update can change the active layer's visibility without a
        // `changed` emission; keep the tool cursor in sync.
        if (tools_) {
            tools_->refreshCursor();
        }
        // Mid-stroke, skip the command-registry and title fan-out on every dab
        // (Krita's "unnecessary objects per event"). The commit presents one
        // region after the stroke session is taken; the debounced pass then
        // refreshes the command table once.
        if (view->is_painting()) {
            // A present arms its own flush: the next event-loop turn folds every
            // dab that arrived since into one region, so the composite and the
            // present track the frame rate, not the input rate.
            paint_api::presentDeferred(*view);
            return;
        }
        if (DocEntry* doc = documents_->docAt(index)) {
            refreshLayerEdges(*doc);
            // A shape or vector-mask target path is drawn from the layer's
            // outline, so a Move or transform present must redraw it too.
            for (const DocWindow* window : doc->windows) {
                if (window) {
                    refreshTargetPathOverlay(*window);
                }
            }
        }
        if (view == activeView()) {
            panels_->setPanelsRefreshedByChanged(false);
        }
        panels_->setPanelsFreshlyBound(false);
        panels_->schedulePanelRefresh();
        updateTabTitle(index);
        updateWindowTitle();
        panels_->setRegistryRefreshPending(true);
    };
    // Document canvases receive the changed rectangle only: their present crop
    // is patched from the pyramid, so no region pixels cross the bridge.
    connect(view, &PictureView::regionDamaged, this,
            [this, view, afterPresent](int x, int y, int w, int h) {
                pictura::ScopedTimer damageSlotTimer("cxx_regionDamaged_slot");
                const int index = indexOfView(view);
                if (index >= 0) {
                    const QRect damaged(x, y, w, h);
                    forEachCanvas(*documents_->docAt(index), [&damaged](ImageView* canvas) {
                        canvas->damageRegion(damaged);
                    });
                }
                afterPresent(index);
            });
    connect(view, &PictureView::pyramid_settled, this, [this, view] {
        const int index = indexOfView(view);
        if (index >= 0) {
            forEachCanvas(*documents_->docAt(index), [](ImageView* canvas) { canvas->update(); });
        }
    });
    // A large Move commit keeps its drag preview while a worker composites; the
    // signal says the composite is current, so the preview can go.
    connect(view, &PictureView::moveCommitted, this, [this, view] {
        const int index = indexOfView(view);
        if (index >= 0) {
            forEachCanvas(*documents_->docAt(index), [](ImageView* canvas) { canvas->endMovePreview(); });
        }
    });
    // A canvas that holds its own image still takes the pixel path (the
    // non-document fallback).
    connect(view, &PictureView::regionBlitted, this,
            [this, view, afterPresent](const QImage& region, int x, int y) {
                pictura::ScopedTimer blitSlotTimer("cxx_regionBlitted_slot");
                const int index = indexOfView(view);
                if (index >= 0) {
                    forEachCanvas(*documents_->docAt(index), [&region, x, y](ImageView* canvas) {
                        canvas->blitRegion(region, x, y);
                    });
                }
                afterPresent(index);
            });
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
    QT_TRANSLATE_NOOP("pictura::PicturaMainWindow", "Custom"), QT_TRANSLATE_NOOP("pictura::PicturaMainWindow", "Dark Gray"), QT_TRANSLATE_NOOP("pictura::PicturaMainWindow", "Black"),
    QT_TRANSLATE_NOOP("pictura::PicturaMainWindow", "White")};

} // namespace

QColor PicturaMainWindow::canvasColorForIndex(int index) const
{
    return index <= 0 ? Theme::workspaceColor() : kCanvasColors[index];
}

void PicturaMainWindow::cycleCanvasColor(bool forward)
{
    canvasColorIndex_ =
        (canvasColorIndex_ + (forward ? 1 : -1) + kCanvasColorCount) % kCanvasColorCount;
    for (const DocWindow* window : documents_->windows()) {
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
            for (const DocWindow* window : documents_->windows()) {
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
    for (const DocWindow* window : documents_->windows()) {
        if (window->canvas) {
            window->canvas->setCanvasColor(color);
        }
    }
}

} // namespace pictura

namespace pictura {

void PicturaMainWindow::showCanvasContextMenu(ImageView* canvas, const QPoint& globalPos)
{
    if (!canvas) {
        return;
    }
    // A right-click routes to the active tool's menu, which may also show over
    // the pasteboard (e.g. the Zoom menu). With no tool menu, the pasteboard
    // falls back to the canvas-background colour menu.
    const QPointF imagePos = canvas->widgetToImage(canvas->mapFromGlobal(globalPos));
    if (tools_ && tools_->routeCanvasContextMenu(canvas, imagePos, globalPos)) {
        return;
    }
    const bool overDocument = canvas->hasDocument()
        && QRectF(QPointF(0, 0), canvas->documentSize()).contains(imagePos);
    if (!overDocument) {
        showCanvasBackgroundMenu(globalPos);
    }
}

bool PicturaMainWindow::releaseIsolation()
{
    if (panels_ && panels_->layersPanel() && panels_->layersPanel()->isolationActive()) {
        panels_->layersPanel()->releaseIsolation();
        return true;
    }
    return false;
}

} // namespace pictura
