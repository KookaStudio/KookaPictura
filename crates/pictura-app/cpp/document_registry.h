#pragma once

#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtCore/QtGlobal>
#include <QtGui/QColor>

#include <functional>
#include <memory>

#include "document_window.h"

class QEvent;
class QMimeData;
class QStackedWidget;
class QTabBar;
class QTabWidget;
class QWidget;

namespace pictura {

class DocumentFloatWindow;
class DocumentTileHost;
class DocumentTilePane;
class ImageView;
class PictureView;

// Owns the open documents, their windows, the active window, and the
// tab/float/tile hosts. The frame keeps the chrome, the view/canvas wiring and
// the dialogs; the hooks below are that side of the seam.
class DocumentRegistry : public QObject {
    Q_OBJECT

public:
    struct Hooks {
        // The frame widget, used as the parent for tabs, floats and overlays.
        std::function<QWidget*()> shellWidget;
        // Preferences and chrome state that decide how a ready window is hosted.
        std::function<bool()> openDocumentsAsTabs;
        std::function<bool()> floatingDocumentDocking;
        std::function<bool()> fullScreen;
        // The frame's presentation refresh and its coalesced panel pass.
        std::function<void()> requestRefresh;
        std::function<void()> requestPanelRefresh;
        std::function<void()> stopPanelRefreshTimer;
        std::function<void()> activateShellWindow;
        std::function<void()> raisePanelOverlays;
        std::function<void()> retargetPanels;
        std::function<void(DocWindow*)> closeWindowRequested;
        // Canvas construction, wiring and view settings stay frame-owned.
        std::function<void(DocWindow&)> setupCanvas;
        std::function<void(DocWindow&)> wireCanvas;
        std::function<void(DocEntry&)> wireDocument;
        std::function<void(PictureView*)> applyViewSettings;
        std::function<void(PictureView*)> wireOpenSignals;
        std::function<void(const QString&)> rememberRecent;
        std::function<void(PictureView*)> forgetToolView;
        std::function<void(ImageView*)> forgetCanvas;
        std::function<quint64()> openDelayForTest;
        std::function<QColor()> loadingPageColor;
        std::function<bool()> scrollAllWindows;
        std::function<ImageView*()> toolCanvas;
    };

    explicit DocumentRegistry(Hooks hooks, QWidget* shell, QObject* parent = nullptr);
    ~DocumentRegistry() override;

    // Hosts.
    QTabWidget* tabs() const { return tabs_; }
    QStackedWidget* documentArea() const { return documentArea_; }
    DocumentTileHost* tileHost() const { return tileHost_; }
    QTabBar* tabBar() const;

    // Document index / active API.
    int count() const { return int(docs_.size()); }
    int activeIndex() const;
    void setActiveIndex(int index);
    DocEntry* docAt(int index) const;
    const QList<DocEntry*>& docs() const { return docs_; }
    QList<DocWindow*>& windows() { return windows_; }
    const QList<DocWindow*>& windows() const { return windows_; }
    DocWindow* activeWindow() const { return activeWindow_; }
    void setTabTrackingSuppressed(bool suppressed);
    PictureView* viewAt(int index) const;
    PictureView* activeView() const;
    ImageView* canvasAt(int index) const;
    ImageView* imageView() const;
    QString documentPath(int index) const;
    QString documentName(int index) const;
    bool isDocumentDirty(int index) const;
    QString activeFilePath() const;
    QString activeDocumentName() const;
    bool isActiveDirty() const;
    bool isLoading(int index) const;
    int untitledCounter() const { return untitledCounter_; }
    void setDocumentName(int index, const QString& name);
    void setDocumentPath(int index, const QString& path);
    int indexOfView(const PictureView* view) const;
    int indexOfDoc(const DocEntry* doc) const;
    DocWindow* windowAtTab(int tab) const;
    int tabOf(const DocWindow* window) const;
    DocWindow* windowOfCanvas(const ImageView* canvas) const;

    // Document lifecycle.
    int addDocument(PictureView* view, const QString& path, bool asTab = false);
    int addLoadingDocument(PictureView* view, const QString& path, const QString& displayName,
                           quint64 token, const QString& recentPath, OpenKind kind);
    void finishLoadingDocument(int index);
    void removeDocument(int index);
    void updateTabTitle(int index);
    void updateAllTabTitles();

    // Window hosts.
    bool addDocumentWindow(DocEntry* doc);
    int addTabWindow(DocWindow* window);
    void installCloseButton(DocWindow* window);
    void hostReadyWindow(DocWindow* window, bool activate);
    void activateDocWindow(DocWindow* window);
    bool floatWindow(DocWindow* window, const QRect& globalGeometry = QRect(),
                     bool activate = true);
    bool dockWindow(DocWindow* window, int index);
    bool tileWindow(DocWindow* window, bool activate = true);
    void untileWindow(DocWindow* window);
    void detachWindows(const QList<DocWindow*>& windows);
    void destroyWindowWidgets(DocWindow* window);
    bool stripOwnsActivation() const;
    void presentWindows();
    void syncScreenModeWindows();
    QRect documentAreaGlobalRect() const;
    QRect clampFloatGeometry(const QRect& geometry) const;
    QRect defaultFloatGeometry() const;
    void raiseDocumentFloat(DocumentFloatWindow* floating);
    QRect dockZoneGlobalRect() const;
    int dockIndexAt(const QPoint& globalPos) const;
    void onFloatDragReleased(DocWindow* window, const QPoint& globalPos);
    void forEachCanvas(const DocEntry& doc, const std::function<void(ImageView*)>& fn) const;
    QList<ImageView*> activeDocumentCanvases() const;

    // Window arrange and multi-window tool options.
    bool addDocumentWindowForTest(DocEntry* doc) { return addDocumentWindow(doc); }
    QRect cascadeGeometry(int slot) const;
    void floatAllWindows();
    void cascadeWindows();
    void tileWindows();
    void matchWindows(bool zoom, bool location);
    void scrollOtherWindows(const ImageView* source, const QPointF& delta,
                            Qt::KeyboardModifiers mods);
    void onToolZoomed(double factor, bool allWindows, bool resizeToFit);
    void resizeWindowToFit(DocWindow* window);

    // Tab tear-out and window dock drag-and-drop.
    void wireDocumentMenus();
    bool tearOutFilter(QEvent* event);
    bool windowDropFilter(QObject* watched, QEvent* event);

    // Document window test hooks.
    DocumentFloatWindow* floatWindowForTest(int index) const;
    bool floatDocumentForTest(int index);
    bool dockDocumentForTest(int index, int tab);
    int tabOfDocumentForTest(int index) const;
    QString documentWindowLabelForTest(int index) const;
    DocumentTilePane* tilePaneForTest(int index) const;
    int windowCountForTest(int index) const;
    ImageView* windowCanvasForTest(int index, int ordinal) const;
    QString windowLabelForTest(int index, int ordinal) const;
    int windowTabForTest(int index, int ordinal) const;
    DocumentFloatWindow* windowFloatForTest(int index, int ordinal) const;
    void activateWindowForTest(int index, int ordinal);
    void setWindowDockDragForTest(std::function<void(std::unique_ptr<QMimeData>)> hook);

Q_SIGNALS:
    void activeDocumentChanged();
    void documentsChanged();
    void documentStateChanged(int index);
    void windowsChanged();
    void aboutToRemoveDocument(int index);
    void tabContextMenuRequested(DocWindow* window, const QPoint& barPos);
    void headerContextMenuRequested(DocWindow* window, const QPoint& globalPos);
    void closeDocumentRequested(int index);
    void cancelOpenRequested(PictureView* view);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void buildDocumentTabs();
    void onTabCurrentChanged(int tab);
    void onTabBarClicked(int tab);
    void onActiveWindowChanged();
    void requestCloseWindow(DocWindow* window);
    void cycleDocumentWindow(int step);
    void wireDocumentCycling();
    bool updateDockIndicator(const QPoint& globalPos);
    void hideDockIndicator();
    void applyPendingDock();
    DocWindow* windowOfDockMime(const QMimeData* mime) const;
    void startWindowDockDrag(DocWindow* window, bool carryWindow = false,
                             const QPoint& grabOffset = QPoint());

    Hooks hooks_;
    // The open documents, in open order; the public index API indexes them.
    QList<DocEntry*> docs_;
    // Every document window in cycle order: the tab windows in their visual
    // order first, then the windows of the other hosts.
    QList<DocWindow*> windows_;
    // The window the commands and tools target; null with no document.
    DocWindow* activeWindow_ = nullptr;
    QTabWidget* tabs_ = nullptr;
    QStackedWidget* documentArea_ = nullptr;
    DocumentTileHost* tileHost_ = nullptr;
    bool suppressTabTracking_ = false;
    bool movingHost_ = false;
    // A tab strip press that may become a tear-out, and the float a tear-out
    // handed the drag to.
    DocWindow* tearWindow_ = nullptr;
    QPoint tearPressPos_;
    QPointer<DocumentFloatWindow> tearFloat_;
    bool tearSynthesizing_ = false;
    QWidget* dockIndicator_ = nullptr;
    DocWindow* pendingDockWindow_ = nullptr;
    int pendingDockIndex_ = -1;
    bool windowDockDragRunning_ = false;
    bool scrollingAllWindows_ = false;
    std::function<void(std::unique_ptr<QMimeData>)> windowDockDragForTest_;
    int untitledCounter_ = 0;
};

} // namespace pictura
