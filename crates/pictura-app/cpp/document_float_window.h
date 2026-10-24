#pragma once

#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtWidgets/QLabel>
#include <QtWidgets/QWidget>

#include <functional>

class QAbstractButton;
class QToolButton;
class QVBoxLayout;

namespace pictura {

// The MIME type of a floating window's Ctrl+drag: it carries the document
// window to the tab strip.
inline constexpr char kDocumentWindowMimeType[] = "application/x-pictura-document-window";

// A document window header's title, shared by the floating window and the tile
// pane: the label elided to the title's own width, then the modified marker,
// which elision never drops. The tooltip holds the full label.
class DocumentWindowTitle : public QLabel {
public:
    explicit DocumentWindowTitle(QWidget* parent);

    void setFullText(const QString& label, bool dirty);
    // The unelided text: the label and, while modified, the marker.
    QString fullText() const { return label_ + marker_; }

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    void elide();

    QString label_;
    QString marker_;
};

// A floating document window: a header (label, modified marker, close
// control) over one document window's page, with a corner resize grip. It is a
// frameless `Qt::Tool` top-level window parented to (so transient for) the
// frame; `PanelFloat::setForceChildOverlayForTest` makes it a child overlay of
// the frame instead, for the offscreen geometry tests. The frame owns the
// document state; the float only reports what the user does to it.
class DocumentFloatWindow : public QWidget {
    Q_OBJECT

public:
    // The header strip's height; Cascade steps by it.
    static constexpr int kHeaderHeight = 24;

    explicit DocumentFloatWindow(QWidget* frame);

    // Host `page` under the header. The page is reparented into the float.
    void setPage(QWidget* page);
    QWidget* page() const { return page_; }

    // The header shows `label` elided to its width, then the modified marker.
    void setLabel(const QString& label, bool dirty);
    // The unelided header text: the label and, while modified, the marker.
    QString headerText() const;
    QWidget* header() const { return header_; }
    QAbstractButton* closeButton() const;

    // Geometry in global coordinates, whichever way the float is hosted.
    QRect globalGeometry() const;
    void setGlobalGeometry(const QRect& geometry);

    // A header drag: `beginDrag` keeps the pointer's offset into the float,
    // `dragTo` moves the float under the pointer, and `endDrag` reports the
    // release when the pointer moved. The tab strip's tear-out drives the same
    // three calls. Past the drag threshold, `dragTo` first offers the move to
    // the window system (`startSystemMove`).
    void beginDrag(const QPoint& globalPos);
    void dragTo(const QPoint& globalPos);
    void endDrag(const QPoint& globalPos);

    // True where a header drag hands a top-level float's move to the window
    // system: on Wayland, where a client can't place its own top-level window,
    // or while the test override is on. X11 has a system move too, but it
    // keeps the motion and release from the client, so there the float moves
    // by hand and a release over the tab strip can dock it.
    static bool movesBySystem();
    static void setSystemMoveForTest(bool on);
    // Hand the drag in progress to the window system's interactive move. True
    // when it took the move; the float then ends its own drag, since no motion
    // or release will reach it. False when `movesBySystem()` is off, the float
    // is a child overlay, or the platform refuses (offscreen always does).
    bool startSystemMove();

    // The window was activated or its header pressed.
    std::function<void()> onActivated;
    std::function<void()> onCloseRequested;
    std::function<void(const QPoint& globalPos)> onContextMenu;
    // A header drag that moved ended at `globalPos`.
    std::function<void(const QPoint& globalPos)> onDragReleased;
    // A Ctrl+drag on the header passed the drag threshold: carry the window
    // as drag-and-drop.
    std::function<void()> onDockDragRequested;
    // The header was double-clicked.
    std::function<void()> onDockRequested;

protected:
    bool event(QEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    QWidget* header_ = nullptr;
    DocumentWindowTitle* title_ = nullptr;
    QToolButton* close_ = nullptr;
    QWidget* body_ = nullptr;
    QVBoxLayout* bodyLayout_ = nullptr;
    QWidget* page_ = nullptr;
    QWidget* grip_ = nullptr;
    bool dragging_ = false;
    bool dragMoved_ = false;
    QPoint dragStart_;
    QPoint grabOffset_;
};

} // namespace pictura
